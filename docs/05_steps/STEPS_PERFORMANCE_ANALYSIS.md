# パフォーマンス改善ガイド（分析・提案）

**所属**: [STEPS_ALL.md](./STEPS_ALL.md) 1.3 パフォーマンスの分析・課題整理ドキュメント。

**「Elixir の真価を引き出す — 並行性・耐障害性・分散処理」**

1.2（クオリティ）までで完成したゲームは、Rust が重い計算を担い、Elixir が司令塔として機能する設計です。  
このドキュメントでは、**現状コードの具体的な改善点**を分析し、Elixir/OTP の強みをさらに引き出す手法を提案します。  
**詳細手順・コード**は [STEPS_PERF.md](./STEPS_PERF.md)（1.3 節）を参照すること。

> **注意**: 本ドキュメントで述べているアーキテクチャ（GameEvents が Elixir/GenServer 側にある構成など）は**当時のもの**です。のちに 1.5.1 にてゲームループは Rust 側へ移行しています。記載内容はその時点の分析・提案として参照してください。

---

## 現状の性能分析

### アーキテクチャ概観

```
┌─────────────────────────────────────────────────────┐
│  BEAM VM (Elixir / OTP)                             │
│                                                     │
│  Game.GameEvents ──── 60Hz tick ──→ NIF (Rust)        │
│  Game.StressMonitor ─ 1Hz ──────→ NIF (Rust)        │
│  Game.Stats ─────── async cast ─→ (pure Elixir)     │
│  Game.InputHandler ─ event ─────→ GameEvents cast     │
│  Game.SpawnSystem ── pure fn ───→ (no state)        │
└─────────────────────────────────────────────────────┘
         ↕ Mutex<GameWorldInner>
┌─────────────────────────────────────────────────────┐
│  Rust NIF (game_native)                             │
│  - physics_step: rayon 並列 AI + 衝突判定           │
│  - SoA (Structure of Arrays) データレイアウト        │
│  - Spatial Hash 衝突判定                            │
└─────────────────────────────────────────────────────┘
```

### 現状の課題

| 課題 | 場所 | 影響度 |
|---|---|---|
| `Mutex<GameWorldInner>` が単一ロック | `lib.rs` | ★★★ |
| `get_enemy_count` を毎フレーム 2 回呼ぶ | `game_loop.ex` + `stress_monitor.ex` | ★★ |
| `find_nearest_enemy` が O(N) 全探索 | `lib.rs:401` | ★★★ |
| `spawn_one` / `spawn_ex` の空きスロット線形探索 | `lib.rs:353, 270` | ★★ |
| `BulletWorld::spawn_ex` の線形スキャン | `lib.rs:270` | ★★ |
| `exclude.contains(&i)` が O(N) | `lib.rs:429` | ★ |
| ゲームイベントが Elixir に届かない | 全体 | ★★★ |
| `Stats.record_kill` が GameEvents から呼ばれていない | `stats.ex` | ★★ |
| `StressMonitor` が `GameEvents` に call して world_ref を取得 | `stress_monitor.ex:109` | ★ |

---

## 改善提案

---

## Perf-1: イベントバス（GenServer + ETS）— Elixir 最重要改善

### 問題

現在、Rust NIF の結果（敵撃破・レベルアップ・ダメージ）は `GameEvents` の中で処理されるが、  
`Game.Stats` には通知されていない。`Stats.record_kill` は定義されているが **一度も呼ばれていない**。

```elixir
# game_loop.ex の physics_step 後 — 撃破情報を取り出す仕組みがない
_frame_id = Game.NifBridge.physics_step(state.world_ref, delta * 1.0)
# ↑ 戻り値を捨てている。イベントが Elixir に届かない。
```

### 解決策: ETS ベースのイベントキュー

Rust 側で「このフレームで起きたイベント」をリストとして返し、  
Elixir 側でパターンマッチしてファンアウトする。

#### Step A: Rust 側 — フレームイベントを返す NIF を追加

```rust
// lib.rs に追加

#[derive(Debug)]
pub enum FrameEvent {
    EnemyKilled { enemy_kind: u8, weapon_kind: u8, x: f32, y: f32 },
    PlayerDamaged { damage: f32 },
    LevelUp { new_level: u32 },
    ItemPickup { item_kind: u8 },
    BossDefeated { boss_kind: u8 },
}

pub struct GameWorldInner {
    // ... 既存フィールド ...
    /// このフレームで発生したイベントのバッファ
    pub frame_events: Vec<FrameEvent>,
}
```

```rust
// physics_step 内の敵撃破処理に追記
// 既存: w.enemies.kill(ei);
// 追加:
w.frame_events.push(FrameEvent::EnemyKilled {
    enemy_kind: w.enemies.kinds[ei] as u8,
    weapon_kind: slot.kind as u8,
    x: w.enemies.positions_x[ei],
    y: w.enemies.positions_y[ei],
});

// NIF 関数: フレームイベントを取り出してクリア
#[rustler::nif]
fn drain_frame_events(world: ResourceArc<GameWorld>) -> Vec<(Atom, u8, u8)> {
    let mut w = world.0.lock().unwrap();
    let events = w.frame_events.drain(..).map(|e| match e {
        FrameEvent::EnemyKilled { enemy_kind, weapon_kind, .. } =>
            (enemy_killed(), enemy_kind, weapon_kind),
        _ => (ok(), 0, 0),
    }).collect();
    events
}
```

#### Step B: Elixir 側 — イベントバスでファンアウト

```elixir
# lib/game/event_bus.ex（新規作成）
defmodule Game.EventBus do
  @moduledoc """
  フレームイベントを受け取り、複数のサブスクライバーに配信する。

  Elixir の強み:
  - プロセスへの非同期メッセージ送信（cast）はノンブロッキング
  - サブスクライバーが遅くてもゲームループに影響しない
  - ETS を使えばサブスクライバー一覧の読み取りがロックフリー
  """

  use GenServer

  def start_link(opts \\ []), do: GenServer.start_link(__MODULE__, opts, name: __MODULE__)

  @doc "サブスクライバーを登録する（PID を受け取るプロセス）"
  def subscribe(pid \\ self()) do
    GenServer.cast(__MODULE__, {:subscribe, pid})
  end

  @doc "イベントリストを全サブスクライバーにブロードキャスト"
  def broadcast(events) when is_list(events) do
    GenServer.cast(__MODULE__, {:broadcast, events})
  end

  @impl true
  def init(_opts) do
    {:ok, %{subscribers: MapSet.new()}}
  end

  @impl true
  def handle_cast({:subscribe, pid}, state) do
    Process.monitor(pid)
    {:noreply, %{state | subscribers: MapSet.put(state.subscribers, pid)}}
  end

  @impl true
  def handle_cast({:broadcast, events}, state) do
    Enum.each(state.subscribers, fn pid ->
      send(pid, {:game_events, events})
    end)
    {:noreply, state}
  end

  @impl true
  def handle_info({:DOWN, _ref, :process, pid, _reason}, state) do
    {:noreply, %{state | subscribers: MapSet.delete(state.subscribers, pid)}}
  end
end
```

```elixir
# game_loop.ex の tick 内に追加
# physics_step の後:
events = Game.NifBridge.drain_frame_events(state.world_ref)
unless events == [] do
  Game.EventBus.broadcast(events)
end
```

```elixir
# stats.ex — EventBus をサブスクライブ
@impl true
def init(_opts) do
  Game.EventBus.subscribe()
  {:ok, initial_state()}
end

@impl true
def handle_info({:game_events, events}, state) do
  new_state = Enum.reduce(events, state, fn
    {:enemy_killed, enemy_kind, weapon_kind}, acc ->
      acc
      |> Map.update(:kills_by_enemy,  %{}, &Map.update(&1, enemy_kind,  1, fn n -> n + 1 end))
      |> Map.update(:kills_by_weapon, %{}, &Map.update(&1, weapon_kind, 1, fn n -> n + 1 end))
      |> Map.update(:total_kills, 0, &(&1 + 1))
    _, acc -> acc
  end)
  {:noreply, new_state}
end
```

### 効果

- `Stats.record_kill` が実際に機能するようになる
- 将来的にリプレイ記録・実績システム・外部ダッシュボードを **ゲームループを変更せずに** 追加できる
- OTP の「関心の分離」を完全に実現

---

## Perf-2: ETS キャッシュで NIF 呼び出し回数を削減

### 問題

`GameEvents` と `StressMonitor` が同じデータを独立して取得している。

```elixir
# game_loop.ex:235
enemy_count = Game.NifBridge.get_enemy_count(state.world_ref)
physics_ms  = Game.NifBridge.get_frame_time_ms(state.world_ref)

# stress_monitor.ex:66-69（別プロセスが同じデータを再取得）
enemy_count   = Game.NifBridge.get_enemy_count(world_ref)
physics_ms    = Game.NifBridge.get_frame_time_ms(world_ref)
bullet_count  = Game.NifBridge.get_bullet_count(world_ref)
{hp, max_hp, score, elapsed_s} = Game.NifBridge.get_hud_data(world_ref)
```

さらに `StressMonitor` は `GameEvents` に `GenServer.call` して `world_ref` を取得するため、  
毎秒 1 回 `GameEvents` のメッセージキューをブロックしている。

### 解決策: ETS テーブルにフレームスナップショットをキャッシュ

```elixir
# lib/game/frame_cache.ex（新規作成）
defmodule Game.FrameCache do
  @moduledoc """
  フレームごとのゲーム状態スナップショットを ETS に書き込む。

  ETS の特性:
  - 書き込みは GameEvents（単一ライター）のみ
  - 読み取りは任意のプロセスからロックフリーで可能
  - プロセスクラッシュ時は ETS テーブルごと消えるが、
    Supervisor が再起動すれば自動的に再作成される

  これにより StressMonitor は GameEvents に call せず、
  ETS から直接データを読み取れる。
  """

  @table :frame_cache

  def init do
    :ets.new(@table, [:named_table, :public, read_concurrency: true])
  end

  @doc "GameEvents が毎フレーム書き込む"
  def put(enemy_count, bullet_count, physics_ms, hud_data) do
    :ets.insert(@table, {:snapshot, %{
      enemy_count:  enemy_count,
      bullet_count: bullet_count,
      physics_ms:   physics_ms,
      hud_data:     hud_data,
      updated_at:   System.monotonic_time(:millisecond),
    }})
  end

  @doc "任意のプロセスがロックフリーで読み取る"
  def get do
    case :ets.lookup(@table, :snapshot) do
      [{:snapshot, data}] -> {:ok, data}
      []                  -> :empty
    end
  end
end
```

```elixir
# game_loop.ex の init に追加
Game.FrameCache.init()

# tick の毎秒ログ部分を置き換え
if rem(state.frame_count, 60) == 0 do
  enemy_count  = Game.NifBridge.get_enemy_count(state.world_ref)
  physics_ms   = Game.NifBridge.get_frame_time_ms(state.world_ref)
  bullet_count = Game.NifBridge.get_bullet_count(state.world_ref)
  hud_data     = Game.NifBridge.get_hud_data(state.world_ref)

  # ETS に書き込む（StressMonitor はここから読む）
  Game.FrameCache.put(enemy_count, bullet_count, physics_ms, hud_data)
  # ... ログ出力 ...
end
```

```elixir
# stress_monitor.ex — ETS から読む（GameEvents への call が不要になる）
defp sample_and_log(state) do
  case Game.FrameCache.get() do
    :empty -> state
    {:ok, %{enemy_count: enemy_count, physics_ms: physics_ms,
            bullet_count: bullet_count, hud_data: hud_data}} ->
      # ... 既存のログ処理 ...
  end
end
```

### 効果

- `StressMonitor` → `GameEvents` への `GenServer.call` が消える
- NIF 呼び出し回数が毎秒 4 回削減（`get_enemy_count` × 2、`get_frame_time_ms` × 2 など）
- `StressMonitor` が `world_ref` を保持する必要がなくなり、設計がシンプルになる

---

## Perf-3: Rust 側 — 空きスロット線形探索を O(1) に改善

### 問題

`BulletWorld::spawn_ex` と `ParticleWorld::spawn_one` は、空きスロットを先頭から線形探索している。  
弾丸・パーティクルが大量に存在するとき、毎フレーム O(N) のスキャンが発生する。

```rust
// lib.rs:270 — 毎回先頭から全スキャン
fn spawn_ex(&mut self, ...) {
    for i in 0..self.positions_x.len() {
        if !self.alive[i] {
            // 見つかったら使う
            return;
        }
    }
    // 見つからなければ push
}
```

### 解決策: フリーリスト（空きインデックスのスタック）

```rust
pub struct BulletWorld {
    // ... 既存フィールド ...
    /// 空きスロットのインデックススタック（O(1) で空きを取得）
    free_list: Vec<usize>,
}

impl BulletWorld {
    fn spawn_ex(&mut self, x: f32, y: f32, vx: f32, vy: f32,
                damage: i32, lifetime: f32, piercing: bool, render_kind: u8) {
        let idx = if let Some(i) = self.free_list.pop() {
            // O(1): フリーリストから取得
            self.positions_x[i]  = x;
            self.positions_y[i]  = y;
            // ... 他フィールドも設定 ...
            self.alive[i]        = true;
            i
        } else {
            // フリーリストが空なら新規追加
            let i = self.positions_x.len();
            self.positions_x.push(x);
            // ... push ...
            i
        };
        self.count += 1;
        let _ = idx;
    }

    pub fn kill(&mut self, i: usize) {
        if self.alive[i] {
            self.alive[i] = false;
            self.count = self.count.saturating_sub(1);
            self.free_list.push(i);  // フリーリストに返却
        }
    }
}
```

同様に `ParticleWorld` と `EnemyWorld` にも適用する。

### 効果

- 弾丸スポーン: O(N) → O(1)
- パーティクルスポーン: O(N) → O(1)
- 敵スポーン: O(N) → O(1)（`EnemyWorld::spawn` の内部ループも同様）
- 10,000 体規模では体感できるレベルの改善

---

## Perf-4: `find_nearest_enemy` を Spatial Hash で O(1) に改善

### 問題

`MagicWand` の自動照準で毎フレーム呼ばれる `find_nearest_enemy` は全敵を線形スキャンする。

```rust
// lib.rs:401 — 10,000 体いれば 10,000 回ループ
pub fn find_nearest_enemy(enemies: &EnemyWorld, px: f32, py: f32) -> Option<usize> {
    for i in 0..enemies.len() {
        // ...
    }
}
```

### 解決策: 既存の `CollisionWorld` (Spatial Hash) を活用

```rust
// lib.rs の physics_step 内で、rebuild_collision() 後に呼ぶ
pub fn find_nearest_enemy_spatial(
    collision: &CollisionWorld,
    enemies: &EnemyWorld,
    px: f32, py: f32,
    search_radius: f32,
) -> Option<usize> {
    // Spatial Hash で半径内の候補だけ取得（通常数十体）
    let candidates = collision.dynamic.query_radius(px, py, search_radius);

    candidates.iter()
        .filter(|&&i| enemies.alive[i])
        .min_by(|&&a, &&b| {
            let da = dist_sq(enemies.positions_x[a], enemies.positions_y[a], px, py);
            let db = dist_sq(enemies.positions_x[b], enemies.positions_y[b], px, py);
            da.partial_cmp(&db).unwrap()
        })
        .copied()
        // 半径内に誰もいなければ全体から探す（フォールバック）
        .or_else(|| find_nearest_enemy(enemies, px, py))
}

fn dist_sq(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let dx = x1 - x2;
    let dy = y1 - y2;
    dx * dx + dy * dy
}
```

### 効果

- 武器が 6 種類 × 複数スロット = 最大 6 回/フレームの `find_nearest_enemy` が高速化
- 10,000 体時: 10,000 回 → 平均 20〜50 回の比較に削減

---

## Perf-5: `exclude.contains` を HashSet で O(1) に改善

### 問題

`find_nearest_enemy_excluding`（Lightning チェーン用）が `exclude.contains(&i)` で O(N) スキャン。

```rust
// lib.rs:429
if !enemies.alive[i] || exclude.contains(&i) {  // Vec の contains は O(N)
```

### 解決策

```rust
use std::collections::HashSet;
// または rustc-hash（既に Cargo.toml に依存関係あり）
use rustc_hash::FxHashSet;

pub fn find_nearest_enemy_excluding(
    enemies: &EnemyWorld,
    px: f32, py: f32,
    exclude: &FxHashSet<usize>,  // Vec → HashSet に変更
) -> Option<usize> {
    // ...
    if !enemies.alive[i] || exclude.contains(&i) {  // O(1)
```

呼び出し側（Lightning 処理）:

```rust
let mut hit_indices = FxHashSet::default();
// チェーン処理のループ内:
hit_indices.insert(target_idx);
find_nearest_enemy_excluding(&enemies, x, y, &hit_indices)
```

---

## Perf-6: Elixir — Task.async_stream で並列スポーン計算

### 問題

現在の `SpawnSystem.maybe_spawn` は単一プロセスで動作し、  
スポーン座標の計算（ランダム生成）を逐次実行している。

### 解決策: Task.async_stream で並列化

```elixir
# lib/game/systems/spawn_system.ex
defmodule Game.SpawnSystem do
  @doc """
  複数の敵タイプを並列にスポーンする（Task.async_stream 版）。

  Elixir の強み:
  - Task.async_stream は BEAM の軽量プロセスで並列実行
  - OS スレッドを使わずに並列性を実現
  - タイムアウト付きで安全（ゲームループをブロックしない）
  """
  def maybe_spawn_parallel(world_ref, elapsed_ms, last_spawn_ms) do
    elapsed_sec = elapsed_ms / 1000.0
    {interval_ms, count} = current_wave(elapsed_sec)

    if elapsed_ms - last_spawn_ms >= interval_ms do
      current = Game.NifBridge.get_enemy_count(world_ref)

      if current < @max_enemies do
        to_spawn = min(count, @max_enemies - current)

        # 複数の敵タイプを並列に決定（純粋計算のみ並列化）
        spawn_plan =
          1..to_spawn
          |> Task.async_stream(
            fn _ -> enemy_kind_for_wave(elapsed_sec) end,
            max_concurrency: System.schedulers_online(),
            timeout: 5
          )
          |> Enum.reduce(%{}, fn {:ok, kind}, acc ->
            Map.update(acc, kind, 1, &(&1 + 1))
          end)

        # NIF 呼び出しはまとめて実行（ロック取得回数を最小化）
        Enum.each(spawn_plan, fn {kind, n} ->
          Game.NifBridge.spawn_enemies(world_ref, kind, n)
        end)
      end

      elapsed_ms
    else
      last_spawn_ms
    end
  end
end
```

> **注意**: `Task.async_stream` のオーバーヘッドは小さいが、  
> スポーン数が少ない（< 10）場合は逐次処理の方が速い。  
> ベンチマークして判断すること。

---

## Perf-7: GenServer の `handle_cast` バッチ処理

### 問題

`InputHandler` は `key_down` / `key_up` のたびに `GameEvents` に `cast` を送る。  
高速なキー入力（ゲームパッド等）では毎フレーム複数の cast が届く可能性がある。

```elixir
# input_handler.ex:46 — キーイベントごとに cast
GenServer.cast(Game.GameEvents, {:input, :move, {dx, dy}})
```

### 解決策: 入力状態のポーリング化

```elixir
# lib/game/input_handler.ex — 変更案
defmodule Game.InputHandler do
  @moduledoc """
  キー入力状態を ETS に書き込む。
  GameEvents は tick のたびに ETS から読み取る（ポーリング）。

  利点:
  - GameEvents のメッセージキューに入力 cast が溜まらない
  - 同一フレーム内の複数キーイベントが自動的にマージされる
  - ETS 読み取りはロックフリー（read_concurrency: true）
  """

  @table :input_state

  def init_ets do
    :ets.new(@table, [:named_table, :public, read_concurrency: true])
    :ets.insert(@table, {:move, {0, 0}})
  end

  @doc "GameEvents が tick のたびに呼ぶ"
  def get_move_vector do
    case :ets.lookup(@table, :move) do
      [{:move, vec}] -> vec
      []             -> {0, 0}
    end
  end

  # handle_cast の中で cast の代わりに ETS 書き込み
  defp notify_game_loop(keys_held) do
    dx = calc_dx(keys_held)
    dy = calc_dy(keys_held)
    :ets.insert(@table, {:move, {dx, dy}})
    # GameEvents への cast は不要になる
  end
end
```

```elixir
# game_loop.ex の tick 内で入力を読む
def handle_info(:tick, state) do
  {dx, dy} = Game.InputHandler.get_move_vector()
  Game.NifBridge.set_player_input(state.world_ref, dx * 1.0, dy * 1.0)
  # ...
end
```

---

## Perf-8: Rust 側 — `physics_step` の `Mutex` を `RwLock` に変更

### 問題

`GameWorld(pub Mutex<GameWorldInner>)` は、読み取り専用の NIF 関数（`get_enemy_count` など）でも  
排他ロックを取得する。`StressMonitor` が読み取り中に `GameEvents` の tick がブロックされる可能性がある。

```rust
// lib.rs:609
pub struct GameWorld(pub Mutex<GameWorldInner>);
```

### 解決策: `RwLock` に変更

```rust
use std::sync::RwLock;

pub struct GameWorld(pub RwLock<GameWorldInner>);

// 書き込み NIF（physics_step, spawn_enemies など）
#[rustler::nif]
fn physics_step(world: ResourceArc<GameWorld>, delta_ms: f64) -> NifResult<u32> {
    let mut w = world.0.write().unwrap();
    // ...
}

// 読み取り NIF（get_enemy_count, get_hud_data など）
#[rustler::nif]
fn get_enemy_count(world: ResourceArc<GameWorld>) -> NifResult<usize> {
    let w = world.0.read().unwrap();  // 共有ロック — 複数プロセスが同時に読める
    Ok(w.enemies.count)
}
```

### 効果

- `StressMonitor` と `GameEvents` が同時に NIF を呼んでも、読み取り系はブロックしない
- `physics_step`（書き込み）は依然として排他ロックを取得するが、読み取り専用 NIF との競合が解消

---

## Perf-9: Elixir — Telemetry で計測基盤を整備

### 概要

`:telemetry` は Erlang/Elixir エコシステム標準の計測ライブラリ。  
ゲームループの各フェーズに計測ポイントを埋め込み、  
将来的に LiveDashboard や Prometheus と連携できる。

```elixir
# mix.exs に追加
{:telemetry, "~> 1.3"},
{:telemetry_metrics, "~> 1.0"},
```

```elixir
# game_loop.ex の tick 内
:telemetry.execute(
  [:game, :tick],
  %{
    physics_ms:   physics_ms,
    enemy_count:  enemy_count,
    frame_count:  state.frame_count,
  },
  %{phase: state.phase}
)
```

```elixir
# lib/game/telemetry.ex（新規作成）
defmodule Game.Telemetry do
  use Supervisor

  def start_link(opts), do: Supervisor.start_link(__MODULE__, opts, name: __MODULE__)

  @impl true
  def init(_opts) do
    children = [
      {Telemetry.Metrics.ConsoleReporter, metrics: metrics()},
    ]
    Supervisor.init(children, strategy: :one_for_one)
  end

  defp metrics do
    [
      Telemetry.Metrics.summary("game.tick.physics_ms",
        unit: {:native, :millisecond},
        description: "Rust physics step duration"
      ),
      Telemetry.Metrics.last_value("game.tick.enemy_count",
        description: "Active enemy count"
      ),
    ]
  end
end
```

---

## Perf-10: Rust 側 — SIMD による AI 計算の高速化（上級）

### 概要

`update_chase_ai` は rayon で並列化済みだが、  
各要素の計算（正規化・乗算）は SIMD で 4〜8 要素を同時処理できる。

```rust
// Cargo.toml に追加
[target.'cfg(target_arch = "x86_64")'.dependencies]
# 標準ライブラリの std::arch を使う（追加依存なし）

// lib.rs — SIMD 版 AI 更新（x86_64 専用）
#[cfg(target_arch = "x86_64")]
pub fn update_chase_ai_simd(enemies: &mut EnemyWorld, player_x: f32, player_y: f32, dt: f32) {
    use std::arch::x86_64::*;
    let len = enemies.len();
    let chunks = len / 4;

    unsafe {
        let px4 = _mm_set1_ps(player_x);
        let py4 = _mm_set1_ps(player_y);
        let dt4 = _mm_set1_ps(dt);

        for chunk in 0..chunks {
            let base = chunk * 4;
            // 4 要素を同時ロード
            let ex = _mm_loadu_ps(enemies.positions_x[base..].as_ptr());
            let ey = _mm_loadu_ps(enemies.positions_y[base..].as_ptr());

            let dx = _mm_sub_ps(px4, ex);
            let dy = _mm_sub_ps(py4, ey);
            let dist_sq = _mm_add_ps(_mm_mul_ps(dx, dx), _mm_mul_ps(dy, dy));
            let inv_dist = _mm_rsqrt_ps(dist_sq);  // 高速逆平方根

            // 速度を計算して位置を更新
            // ...
        }
        // 残り要素をスカラーで処理
    }
}
```

> **注意**: SIMD は `#[cfg(target_arch)]` でアーキテクチャを限定し、  
> フォールバックとして既存の rayon 版を残すこと。

---

## 改善優先度まとめ

| 優先度 | 改善項目 | 難易度 | 効果 | Elixir の真価 |
|---|---|---|---|---|
| ★★★ | **Perf-1**: イベントバス | 中 | ★★★ | ◎ OTP の関心分離 |
| ★★★ | **Perf-3**: フリーリスト | 低 | ★★★ | — Rust 側改善 |
| ★★★ | **Perf-4**: Spatial Hash 最近接 | 中 | ★★★ | — Rust 側改善 |
| ★★ | **Perf-2**: ETS キャッシュ | 低 | ★★ | ◎ ETS の活用 |
| ★★ | **Perf-5**: HashSet exclude | 低 | ★ | — Rust 側改善 |
| ★★ | **Perf-8**: RwLock | 低 | ★★ | — Rust 側改善 |
| ★ | **Perf-6**: Task.async_stream | 中 | ★ | ◎ 並列プロセス |
| ★ | **Perf-7**: 入力ポーリング化 | 低 | ★ | ◎ ETS 活用 |
| ★ | **Perf-9**: Telemetry | 低 | ★★ | ◎ 観測可能性 |
| ○ | **Perf-10**: SIMD | 高 | ★★★ | — Rust 上級 |

---

## Elixir の真価を引き出すポイント総括

このゲームエンジンで Elixir が本当に輝く場面は次の 3 つです。

### 1. 並行性（Concurrency）

```
GameEvents ─── 60Hz ──→ Rust NIF
StressMonitor ─ 1Hz ──→ ETS（ロックフリー読み取り）
Stats ──────── async ─→ EventBus 経由でイベント受信
Telemetry ─── async ─→ 計測データを非同期集計
```

各プロセスは **完全に独立** して動作し、互いをブロックしない。  
これは OS スレッドではなく BEAM の軽量プロセス（数KB/プロセス）で実現される。

### 2. 耐障害性（Fault Tolerance）

```elixir
# application.ex — one_for_one 戦略
# Stats がクラッシュしても GameEvents は止まらない
# StressMonitor がクラッシュしても GameEvents は止まらない
# EventBus がクラッシュしても Supervisor が即座に再起動
```

Rust 側でパニックが起きても、NIF は BEAM プロセスを巻き込まない  
（`ResourceArc` + `Mutex` で安全に隔離されている）。

### 3. 観測可能性（Observability）

```elixir
# Telemetry + ETS により、実行中のゲームを外部から観察できる
# 将来的に Phoenix LiveDashboard と接続すれば
# ブラウザからリアルタイムでゲーム状態を監視できる
iex> Game.Stats.session_summary()
iex> Game.StressMonitor.get_stats()
iex> :ets.lookup(:frame_cache, :snapshot)
```

---

## 実装ロードマップ

```
Perf-1: イベントバス実装（Stats が実際に機能するように）
  ↓
Perf-2: ETS キャッシュ（StressMonitor の call を排除）
  ↓
Perf-3: フリーリスト（Rust 側、スポーン O(1) 化）
  ↓
Perf-4: Spatial Hash 最近接（Rust 側、AI 高速化）
  ↓
Perf-5: HashSet exclude（Rust 側、小改善）
Perf-7: 入力ポーリング化（Elixir 側）
Perf-8: RwLock（Rust 側）
  ↓
Perf-9: Telemetry 計測基盤
  ↓
Perf-10: SIMD（オプション、上級者向け）
```
