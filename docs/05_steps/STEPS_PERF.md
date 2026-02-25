# 1.3 パフォーマンス（全6項）

**所属**: [STEPS_ALL.md](./STEPS_ALL.md) 1章 ゲームエンジン基礎 の 1.3 節。

**「Elixir の真価を引き出す — 並行性・耐障害性・観測可能性」**

1.2（クオリティ）までで完成したゲームをベースに、**Elixir/OTP の強みを最大限に引き出す**改善を  
ひとつずつ丁寧に実装するためのステップガイドです。  
各項は独立して動作確認できる単位に分割されています。

---

## 1.3 節 全体ロードマップ（1.3.1〜1.3.6）

| 項 | 目標 | 備考 |
|----|------|------|
| 1.3.1 | イベントバス | フレームイベントを Elixir に配信 |
| 1.3.2 | ETS キャッシュ・入力ポーリング | プロセス間通信の最適化 |
| 1.3.3 | フリーリスト（スポーン O(1)） | P3 |
| 1.3.4 | Spatial Hash 最近接・RwLock | AI 高速化（P1, P2） |
| 1.3.5 | Telemetry 計測基盤 | 観測可能性（P7） |
| 1.3.6 | SIMD AI 高速化（オプション） | P4 |

**推奨順（パフォーマンス優先）**: 4. → 3. → 1. → 2. → 5. → 6.  
**詳細**: [STEPS_PERFORMANCE_ANALYSIS.md](./STEPS_PERFORMANCE_ANALYSIS.md)、[PRIORITY_STEPS.md](../04_roadmap/PRIORITY_STEPS.md)

---

## なぜこの順番か（PRIORITY_STEPS 準拠）

**パフォーマンス最優先**の場合:

```
1.3.4 ─── find_nearest_enemy + Lightning の O(n) を解消
              ↓ 敵数 1000 体以上で最大のボトルネック
1.3.3 ─── スポーン O(n) → O(1) 化
              ↓ 大量エンティティ時の体感改善
1.3.1 ─── イベントバスで Stats を有効化・拡張基盤を整備
              ↓ 以降「ゲームループを触らずに」機能追加可能
1.3.2 ─── ETS でプロセス間通信をロックフリー化
1.3.5 ─── Telemetry で計測基盤を整備
1.3.6 ─── SIMD（オプション）
```

**EventBus ファースト**（拡張性を最優先する場合）:

```
1.3.1 ─── イベントバスを先に作る
              ↓ EventBus があれば、以降の全ステップで
              ↓ 「ゲームループを変更せずに機能追加」できる
1.3.2 ─── ETS でプロセス間通信をロックフリー化
1.3.3/1.3.4 ─ Rust 側の計算量削減
1.3.5 ─── 改善効果を数値で確認する計測基盤
1.3.6 ─── さらなる高速化（オプション）
```

---

## 1.3.1 イベントバス

**PRIORITY_STEPS**: P5

### 目標

Rust の物理ステップで発生したイベント（敵撃破・ダメージ・レベルアップ）を  
Elixir 側の複数プロセスに**ノンブロッキングで配信**する仕組みを作る。

### なぜ重要か

現状、`Game.Stats` の `record_kill` は定義されているが **一度も呼ばれていない**。  
`physics_step` の戻り値は `_frame_id` として捨てられており、  
Rust 側で何が起きたかが Elixir に届いていない。

```elixir
# game_loop.ex:159 — 現状: イベントを捨てている
_frame_id = Game.NifBridge.physics_step(state.world_ref, delta * 1.0)
```

EventBus を入れると、以降のステップで「リプレイ記録」「実績システム」「外部ダッシュボード」を  
**ゲームループに一切触れずに** 追加できるようになる。

### 実装内容

#### 26.1 Rust 側: フレームイベントバッファを追加

```rust
// native/game_native/src/lib.rs

/// フレーム内で発生したゲームイベント
#[derive(Debug, Clone)]
pub enum FrameEvent {
    EnemyKilled  { enemy_kind: u8, weapon_kind: u8 },
    PlayerDamaged { damage: f32 },
    LevelUp      { new_level: u32 },
    ItemPickup   { item_kind: u8 },
    BossDefeated { boss_kind: u8 },
}

pub struct GameWorldInner {
    // ... 既存フィールド（変更なし）...

    /// このフレームで発生したイベントのバッファ（毎フレーム drain される）
    pub frame_events: Vec<FrameEvent>,
}
```

`create_world` の初期化に `frame_events: Vec::new()` を追加する。

#### 26.2 Rust 側: 敵撃破時にイベントを積む

`physics_step` 内の敵撃破処理（`w.enemies.kill(ei)` を呼ぶ箇所）に追記する。  
MagicWand / Axe / Cross / Whip / Fireball / Lightning の各武器で共通。

```rust
// 既存の kill 処理の直後に追加（MagicWand の例）
w.enemies.kill(ei);
w.frame_events.push(FrameEvent::EnemyKilled {
    enemy_kind:  w.enemies.kinds[ei] as u8,
    weapon_kind: kind as u8,   // WeaponKind as u8
});
```

プレイヤーダメージ時:

```rust
// 既存の hp 減算の直後に追加
w.player.hp = (w.player.hp - kind.damage_per_sec() * dt).max(0.0);
w.frame_events.push(FrameEvent::PlayerDamaged {
    damage: kind.damage_per_sec() * dt,
});
```

ボス撃破時:

```rust
// ボス HP が 0 以下になった処理の直後
w.frame_events.push(FrameEvent::BossDefeated {
    boss_kind: boss.kind as u8,
});
```

#### 26.3 Rust 側: `drain_frame_events` NIF を追加

```rust
// native/game_native/src/lib.rs

rustler::atoms! {
    // 既存のアトムに追加
    enemy_killed,
    player_damaged,
    level_up_event,
    item_pickup,
    boss_defeated,
}

/// フレームイベントを取り出してクリアする（毎フレーム GameEvents が呼ぶ）
/// 戻り値: [{:enemy_killed, enemy_kind, weapon_kind}, {:player_damaged, damage_x100}, ...]
#[rustler::nif]
fn drain_frame_events(world: ResourceArc<GameWorld>) -> Vec<(Atom, u8, u8)> {
    let mut w = world.0.lock().unwrap();
    w.frame_events
        .drain(..)
        .map(|e| match e {
            FrameEvent::EnemyKilled { enemy_kind, weapon_kind } =>
                (enemy_killed(), enemy_kind, weapon_kind),
            FrameEvent::PlayerDamaged { damage } =>
                (player_damaged(), (damage * 100.0) as u8, 0),
            FrameEvent::LevelUp { new_level } =>
                (level_up_event(), new_level as u8, 0),
            FrameEvent::ItemPickup { item_kind } =>
                (item_pickup(), item_kind, 0),
            FrameEvent::BossDefeated { boss_kind } =>
                (boss_defeated(), boss_kind, 0),
        })
        .collect()
}
```

`rustler::init!` のリストに `drain_frame_events` を追加する。

#### 26.4 Elixir 側: `NifBridge` にスタブを追加

```elixir
# lib/game/nif_bridge.ex に追加
# フレームイベントを取り出す（[{event_atom, arg1, arg2}] のリスト）
def drain_frame_events(_world), do: :erlang.nif_error(:nif_not_loaded)
```

#### 26.5 Elixir 側: `EventBus` GenServer を新規作成

```elixir
# lib/game/event_bus.ex（新規作成）
defmodule Game.EventBus do
  @moduledoc """
  フレームイベントを受け取り、複数のサブスクライバーに配信する。

  Elixir/OTP の強みを体現するモジュール:
  - プロセスへの send はノンブロッキング — GameEvents を止めない
  - サブスクライバーが重い処理をしても GameEvents に影響しない
  - Process.monitor でサブスクライバーの死活を自動監視
  - EventBus 自体がクラッシュしても Supervisor が即座に再起動し、
    GameEvents は一切影響を受けない（one_for_one 戦略）
  """

  use GenServer
  require Logger

  # ── Public API ────────────────────────────────────────────────

  def start_link(opts \\ []), do: GenServer.start_link(__MODULE__, opts, name: __MODULE__)

  @doc "呼び出したプロセスをサブスクライバーとして登録する"
  def subscribe(pid \\ self()) do
    GenServer.cast(__MODULE__, {:subscribe, pid})
  end

  @doc "イベントリストを全サブスクライバーにブロードキャストする"
  def broadcast(events) when is_list(events) do
    GenServer.cast(__MODULE__, {:broadcast, events})
  end

  # ── GenServer callbacks ────────────────────────────────────────

  @impl true
  def init(_opts) do
    {:ok, %{subscribers: MapSet.new()}}
  end

  @impl true
  def handle_cast({:subscribe, pid}, state) do
    Process.monitor(pid)
    Logger.debug("[EventBus] Subscriber registered: #{inspect(pid)}")
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
    Logger.debug("[EventBus] Subscriber down: #{inspect(pid)}")
    {:noreply, %{state | subscribers: MapSet.delete(state.subscribers, pid)}}
  end
end
```

#### 26.6 Elixir 側: `GameEvents` でイベントを drain して broadcast

```elixir
# lib/game/game_loop.ex — physics_step の直後に追加（handle_info :tick の playing フェーズ）

# 変更前:
_frame_id = Game.NifBridge.physics_step(state.world_ref, delta * 1.0)

# 変更後:
_frame_id = Game.NifBridge.physics_step(state.world_ref, delta * 1.0)
events = Game.NifBridge.drain_frame_events(state.world_ref)
unless events == [] do
  Game.EventBus.broadcast(events)
end
```

#### 26.7 Elixir 側: `Stats` が EventBus をサブスクライブ

```elixir
# lib/game/stats.ex — init と handle_info を変更

@impl true
def init(_opts) do
  # EventBus にサブスクライブ — 以降はイベントが自動的に届く
  Game.EventBus.subscribe()
  {:ok, initial_state()}
end

@impl true
def handle_info({:game_events, events}, state) do
  new_state =
    Enum.reduce(events, state, fn
      {:enemy_killed, enemy_kind, weapon_kind}, acc ->
        acc
        |> Map.update!(:kills_by_enemy,  &Map.update(&1, enemy_kind,  1, fn n -> n + 1 end))
        |> Map.update!(:kills_by_weapon, &Map.update(&1, weapon_kind, 1, fn n -> n + 1 end))
        |> Map.update!(:total_kills, &(&1 + 1))

      {:level_up_event, new_level, _}, acc ->
        Map.update!(acc, :max_level_reached, &max(&1, new_level))

      {:item_pickup, item_kind, _}, acc ->
        Map.update!(acc, :items_collected, &Map.update(&1, item_kind, 1, fn n -> n + 1 end))

      _, acc ->
        acc
    end)

  {:noreply, new_state}
end
```

#### 26.8 `Application` に `EventBus` を追加

```elixir
# lib/game/application.ex

children = [
  Game.InputHandler,
  Game.EventBus,      # ← 追加（GameEvents より前に起動）
  Game.GameEvents,
  Game.StressMonitor,
  Game.Stats,
]
```

### 確認ポイント

- [ ] `iex> Game.Stats.session_summary()` で `total_kills` が増えている
- [ ] `kills_by_enemy` / `kills_by_weapon` にデータが入っている
- [ ] `EventBus` がクラッシュしても GameEvents が止まらない（`kill(pid, :kill)` でテスト）
- [ ] ゲームオーバー後に `session_summary` でセッション統計が確認できる

---

## 1.3.2 ETS キャッシュ・入力ポーリング

**PRIORITY_STEPS**: P6

### 目標

プロセス間通信のボトルネックを 2 つ同時に解消する。

1. `StressMonitor` → `GameEvents` への `GenServer.call`（毎秒 1 回、GameEvents をブロック）
2. `InputHandler` → `GameEvents` への `GenServer.cast`（キーイベントごと、メッセージキューを圧迫）

### なぜ重要か

```
現状の問題:
  StressMonitor ──call──→ GameEvents（world_ref 取得のためにブロック）
                ──call──→ NIF × 4（同じデータを再取得）

  InputHandler ──cast──→ GameEvents（キーイベントごとにメッセージ送信）
```

ETS（Erlang Term Storage）はプロセス間でロックフリーに共有できるインメモリストア。  
「書き込みは 1 プロセス、読み取りは複数プロセスが同時に行う」パターンに最適。

### 実装内容

#### 27.1 `FrameCache` モジュールを新規作成

```elixir
# lib/game/frame_cache.ex（新規作成）
defmodule Game.FrameCache do
  @moduledoc """
  フレームごとのゲーム状態スナップショットを ETS に書き込む。

  ETS の特性:
  - 書き込みは GameEvents（単一ライター）のみ — 競合なし
  - 読み取りは任意のプロセスからロックフリーで可能
  - read_concurrency: true で並列読み取りを最適化
  - GameEvents がクラッシュして ETS テーブルが消えても、
    Supervisor 再起動後に GameEvents.init/1 で再作成される
  """

  @table :frame_cache

  @doc "GameEvents.init/1 から呼ぶ — ETS テーブルを作成する"
  def init do
    :ets.new(@table, [:named_table, :public, :set, read_concurrency: true])
  end

  @doc "GameEvents が毎秒（60 フレームごと）書き込む"
  def put(enemy_count, bullet_count, physics_ms, hud_data) do
    :ets.insert(@table, {:snapshot, %{
      enemy_count:  enemy_count,
      bullet_count: bullet_count,
      physics_ms:   physics_ms,
      hud_data:     hud_data,
      updated_at:   System.monotonic_time(:millisecond),
    }})
  end

  @doc "StressMonitor など任意のプロセスがロックフリーで読み取る"
  def get do
    case :ets.lookup(@table, :snapshot) do
      [{:snapshot, data}] -> {:ok, data}
      []                  -> :empty
    end
  end
end
```

#### 27.2 `GameEvents` を修正: ETS に書き込む

```elixir
# lib/game/game_loop.ex

# init/1 に追加（world_ref 取得の後）
Game.FrameCache.init()

# handle_info :tick の毎秒ログ部分を修正
if rem(state.frame_count, 60) == 0 do
  enemy_count  = Game.NifBridge.get_enemy_count(state.world_ref)
  physics_ms   = Game.NifBridge.get_frame_time_ms(state.world_ref)
  bullet_count = Game.NifBridge.get_bullet_count(state.world_ref)
  hud_data     = Game.NifBridge.get_hud_data(state.world_ref)

  # ETS に書き込む（StressMonitor はここから読む — NIF 呼び出しの重複がなくなる）
  Game.FrameCache.put(enemy_count, bullet_count, physics_ms, hud_data)

  # 以降のログ出力は変更なし
  {_hp, _max_hp, _score, elapsed_s} = hud_data
  wave = Game.SpawnSystem.wave_label(elapsed_s)
  # ...
end
```

また、`handle_call(:get_world_ref, ...)` は不要になるため削除する。

#### 27.3 `StressMonitor` を修正: ETS から読む

```elixir
# lib/game/stress_monitor.ex

# get_world_ref/0 プライベート関数を削除

defp sample_and_log(state) do
  # GameEvents への call が不要になる
  case Game.FrameCache.get() do
    :empty ->
      state

    {:ok, %{
      enemy_count:  enemy_count,
      bullet_count: bullet_count,
      physics_ms:   physics_ms,
      hud_data:     {hp, max_hp, score, elapsed_s},
    }} ->
      overrun = physics_ms > @frame_budget_ms

      new_state = %{state |
        samples:          state.samples + 1,
        peak_enemies:     Kernel.max(state.peak_enemies, enemy_count),
        peak_physics_ms:  Float.round(Kernel.max(state.peak_physics_ms, physics_ms), 2),
        overrun_count:    state.overrun_count + if(overrun, do: 1, else: 0),
        last_enemy_count: enemy_count,
      }

      # 以降のログ出力は変更なし
      wave = Game.SpawnSystem.wave_label(elapsed_s)
      # ...
      new_state
  end
end
```

#### 27.4 `InputHandler` を修正: ETS ポーリング化

```elixir
# lib/game/input_handler.ex

defmodule Game.InputHandler do
  @moduledoc """
  キー入力状態を ETS に書き込む。
  GameEvents は tick のたびに ETS から読み取る（ポーリング方式）。

  変更前: キーイベントごとに GameEvents へ cast → メッセージキューに溜まる
  変更後: ETS に書き込むだけ → GameEvents は tick 時に 1 回だけ読む

  同一フレーム内の複数キーイベントは ETS の上書きで自動マージされる。
  """

  use GenServer

  @table :input_state

  def start_link(opts), do: GenServer.start_link(__MODULE__, opts, name: __MODULE__)

  @doc "GameEvents が tick のたびに呼ぶ — ロックフリー読み取り"
  def get_move_vector do
    case :ets.lookup(@table, :move) do
      [{:move, vec}] -> vec
      []             -> {0, 0}
    end
  end

  def key_down(key), do: GenServer.cast(__MODULE__, {:key_down, key})
  def key_up(key),   do: GenServer.cast(__MODULE__, {:key_up, key})

  @impl true
  def init(_opts) do
    :ets.new(@table, [:named_table, :public, :set, read_concurrency: true])
    :ets.insert(@table, {:move, {0, 0}})
    {:ok, %{keys_held: MapSet.new()}}
  end

  @impl true
  def handle_cast({:key_down, key}, state) do
    if MapSet.member?(state.keys_held, key) do
      {:noreply, state}
    else
      new_keys = MapSet.put(state.keys_held, key)
      write_move_vector(new_keys)
      {:noreply, %{state | keys_held: new_keys}}
    end
  end

  @impl true
  def handle_cast({:key_up, key}, state) do
    if MapSet.member?(state.keys_held, key) do
      new_keys = MapSet.delete(state.keys_held, key)
      write_move_vector(new_keys)
      {:noreply, %{state | keys_held: new_keys}}
    else
      {:noreply, state}
    end
  end

  defp write_move_vector(keys_held) do
    dx =
      (if MapSet.member?(keys_held, :d) or MapSet.member?(keys_held, :arrow_right), do: 1, else: 0) +
      (if MapSet.member?(keys_held, :a) or MapSet.member?(keys_held, :arrow_left),  do: -1, else: 0)

    dy =
      (if MapSet.member?(keys_held, :s) or MapSet.member?(keys_held, :arrow_down), do: 1, else: 0) +
      (if MapSet.member?(keys_held, :w) or MapSet.member?(keys_held, :arrow_up),   do: -1, else: 0)

    # GenServer.cast の代わりに ETS 書き込み（ノンブロッキング）
    :ets.insert(@table, {:move, {dx, dy}})
  end
end
```

#### 27.5 `GameEvents` を修正: 入力を ETS から読む

```elixir
# lib/game/game_loop.ex

# handle_cast({:input, :move, ...}) を削除

# handle_info(:tick, state) の先頭に追加
def handle_info(:tick, state) do
  now   = now_ms()
  delta = now - state.last_tick

  # ETS からロックフリーで入力を読む（cast が届くのを待たない）
  {dx, dy} = Game.InputHandler.get_move_vector()
  Game.NifBridge.set_player_input(state.world_ref, dx * 1.0, dy * 1.0)

  # 以降は変更なし
  _frame_id = Game.NifBridge.physics_step(state.world_ref, delta * 1.0)
  # ...
end
```

### 確認ポイント

- [ ] プレイヤーが WASD / 矢印キーで正常に移動する
- [ ] `StressMonitor` のログが引き続き毎秒出力される
- [ ] `iex> :ets.lookup(:frame_cache, :snapshot)` でスナップショットが確認できる
- [ ] `iex> :ets.lookup(:input_state, :move)` で現在の入力ベクトルが確認できる
- [ ] `StressMonitor` を `kill` しても GameEvents が止まらない

---

## 1.3.3 フリーリスト（スポーン O(1)）

**PRIORITY_STEPS**: P3

### 目標

`BulletWorld` / `ParticleWorld` / `EnemyWorld` の空きスロット探索を  
O(N) の線形スキャンから O(1) のフリーリスト方式に変更する。

### なぜ重要か

```rust
// 現状: 毎回先頭から全スキャン（弾丸 500 本 + パーティクル 1000 個 = 1500 回/スポーン）
fn spawn_ex(&mut self, ...) {
    for i in 0..self.positions_x.len() {   // ← O(N)
        if !self.alive[i] { ... return; }
    }
}
```

10,000 体 + 武器 6 種 + パーティクル大量発生の状況では、  
毎フレーム数千回の無駄なスキャンが発生している。

### 実装内容

#### 28.1 `BulletWorld` にフリーリストを追加

```rust
// native/game_native/src/lib.rs

pub struct BulletWorld {
    pub positions_x:  Vec<f32>,
    pub positions_y:  Vec<f32>,
    pub velocities_x: Vec<f32>,
    pub velocities_y: Vec<f32>,
    pub damage:       Vec<i32>,
    pub lifetime:     Vec<f32>,
    pub alive:        Vec<bool>,
    pub piercing:     Vec<bool>,
    pub render_kind:  Vec<u8>,
    pub count:        usize,
    /// 空きスロットのインデックススタック — O(1) でスロットを取得・返却
    free_list:        Vec<usize>,
}

impl BulletWorld {
    pub fn new() -> Self {
        Self {
            // ... 既存フィールド（変更なし）...
            free_list: Vec::new(),
        }
    }

    fn spawn_ex(&mut self, x: f32, y: f32, vx: f32, vy: f32,
                damage: i32, lifetime: f32, piercing: bool, render_kind: u8) {
        if let Some(i) = self.free_list.pop() {
            // O(1): フリーリストから空きスロットを取得
            self.positions_x[i]  = x;
            self.positions_y[i]  = y;
            self.velocities_x[i] = vx;
            self.velocities_y[i] = vy;
            self.damage[i]       = damage;
            self.lifetime[i]     = lifetime;
            self.alive[i]        = true;
            self.piercing[i]     = piercing;
            self.render_kind[i]  = render_kind;
        } else {
            // フリーリストが空なら末尾に追加
            self.positions_x.push(x);
            self.positions_y.push(y);
            self.velocities_x.push(vx);
            self.velocities_y.push(vy);
            self.damage.push(damage);
            self.lifetime.push(lifetime);
            self.alive.push(true);
            self.piercing.push(piercing);
            self.render_kind.push(render_kind);
        }
        self.count += 1;
    }

    pub fn kill(&mut self, i: usize) {
        if self.alive[i] {
            self.alive[i] = false;
            self.count = self.count.saturating_sub(1);
            self.free_list.push(i);   // ← フリーリストに返却
        }
    }
}
```

#### 28.2 `ParticleWorld` にフリーリストを追加

```rust
pub struct ParticleWorld {
    // ... 既存フィールド ...
    free_list: Vec<usize>,
}

impl ParticleWorld {
    pub fn spawn_one(&mut self, x: f32, y: f32, vx: f32, vy: f32,
                     lifetime: f32, color: [f32; 4], size: f32) {
        if let Some(i) = self.free_list.pop() {
            self.positions_x[i]  = x;
            self.positions_y[i]  = y;
            self.velocities_x[i] = vx;
            self.velocities_y[i] = vy;
            self.lifetime[i]     = lifetime;
            self.max_lifetime[i] = lifetime;
            self.color[i]        = color;
            self.size[i]         = size;
            self.alive[i]        = true;
        } else {
            self.positions_x.push(x);
            self.positions_y.push(y);
            self.velocities_x.push(vx);
            self.velocities_y.push(vy);
            self.lifetime.push(lifetime);
            self.max_lifetime.push(lifetime);
            self.color.push(color);
            self.size.push(size);
            self.alive.push(true);
        }
        self.count += 1;
    }

    pub fn kill(&mut self, i: usize) {
        if self.alive[i] {
            self.alive[i] = false;
            self.count = self.count.saturating_sub(1);
            self.free_list.push(i);
        }
    }
}
```

#### 28.3 `EnemyWorld` にフリーリストを追加

`EnemyWorld::spawn` の内部ループも同様に変更する。  
`spawn` は複数座標をまとめて受け取るため、ループ内でフリーリストを使う。

```rust
pub struct EnemyWorld {
    // ... 既存フィールド ...
    free_list: Vec<usize>,
}

impl EnemyWorld {
    pub fn spawn(&mut self, positions: &[(f32, f32)], kind: EnemyKind) {
        let speed  = kind.speed();
        let max_hp = kind.max_hp();

        for &(x, y) in positions {
            if let Some(i) = self.free_list.pop() {
                // O(1): フリーリストから再利用
                self.positions_x[i]  = x;
                self.positions_y[i]  = y;
                self.velocities_x[i] = 0.0;
                self.velocities_y[i] = 0.0;
                self.speeds[i]       = speed;
                self.hp[i]           = max_hp;
                self.alive[i]        = true;
                self.kinds[i]        = kind;
            } else {
                self.positions_x.push(x);
                self.positions_y.push(y);
                self.velocities_x.push(0.0);
                self.velocities_y.push(0.0);
                self.speeds.push(speed);
                self.hp.push(max_hp);
                self.alive.push(true);
                self.kinds.push(kind);
                self.sep_x.push(0.0);
                self.sep_y.push(0.0);
            }
            self.count += 1;
        }
    }

    pub fn kill(&mut self, i: usize) {
        if self.alive[i] {
            self.alive[i] = false;
            self.count = self.count.saturating_sub(1);
            self.free_list.push(i);
        }
    }
}
```

### 確認ポイント

- [ ] 10,000 体 + 大量パーティクルでも 60 FPS を維持する
- [ ] `StressMonitor` の `physics_ms` が改善前より低下している
- [ ] 長時間プレイ後もメモリが増え続けない（スロット再利用の確認）

---

## 1.3.4 Spatial Hash 最近接・RwLock

**PRIORITY_STEPS**: P1（空間ハッシュ化）, P2（RwLock）

### 目標

1. `find_nearest_enemy`（O(N) 全探索）を既存の Spatial Hash を活用して高速化する
2. **Lightning チェーン探索**も空間ハッシュで候補を絞る（最大 6 チェーン × O(n) → O(数十)）
3. `Mutex` を `RwLock` に変更し、読み取り専用 NIF の競合を解消する

### なぜ重要か

```rust
// 現状: MagicWand / Fireball / Lightning が毎フレーム呼ぶ
// 10,000 体いれば 10,000 回ループ × 最大 3 武器 = 30,000 回/フレーム
pub fn find_nearest_enemy(enemies: &EnemyWorld, px: f32, py: f32) -> Option<usize> {
    for i in 0..enemies.len() { ... }   // O(N)
}
```

**Lightning** は連鎖ごとに `find_nearest_enemy` 相当の探索を行うため、  
最大 6 チェーン × 全敵数 = さらに大きなボトルネックになっている。

Spatial Hash はすでに `rebuild_collision()` で毎フレーム更新されている。  
これを最近接探索にも活用すれば、候補を数十体に絞れる。

### 実装内容

#### 29.1 `find_nearest_enemy_spatial` を追加

```rust
// native/game_native/src/lib.rs

/// Spatial Hash を使った高速最近接探索
/// search_radius 内に候補がいなければ全探索にフォールバック
pub fn find_nearest_enemy_spatial(
    collision: &CollisionWorld,
    enemies:   &EnemyWorld,
    px: f32, py: f32,
    search_radius: f32,
) -> Option<usize> {
    let candidates = collision.dynamic.query_nearby(px, py, search_radius);

    let result = candidates.iter()
        .filter(|&&i| enemies.alive[i])
        .min_by(|&&a, &&b| {
            let da = dist_sq(enemies.positions_x[a], enemies.positions_y[a], px, py);
            let db = dist_sq(enemies.positions_x[b], enemies.positions_y[b], px, py);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .copied();

    // 半径内に誰もいなければ全探索（フォールバック）
    result.or_else(|| find_nearest_enemy(enemies, px, py))
}

#[inline]
fn dist_sq(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let dx = x1 - x2;
    let dy = y1 - y2;
    dx * dx + dy * dy
}
```

#### 29.2 `physics_step` 内の呼び出しを切り替える

```rust
// physics_step 内の武器処理（rebuild_collision() の後）

// 変更前:
if let Some(ti) = find_nearest_enemy(&w.enemies, px, py) { ... }

// 変更後（MagicWand / Fireball / Lightning の find_nearest_enemy をすべて置き換え）:
// search_radius: 画面の半分程度（640px）を基準にする
let search_radius = 640.0_f32;
if let Some(ti) = find_nearest_enemy_spatial(&w.collision, &w.enemies, px, py, search_radius) { ... }
```

**Lightning チェーン**の最適化: 連鎖ごとに `find_nearest_enemy_spatial` を呼ぶ際、  
既にヒットした敵のインデックスを `exclude: &[usize]` で渡すバリアントを追加するか、  
`query_nearby` の結果からヒット済みをフィルタする。

#### 29.3 `Mutex` を `RwLock` に変更

```rust
// native/game_native/src/lib.rs

use std::sync::RwLock;

// 変更前:
pub struct GameWorld(pub Mutex<GameWorldInner>);

// 変更後:
pub struct GameWorld(pub RwLock<GameWorldInner>);
```

書き込み NIF（`physics_step`, `spawn_enemies`, `add_weapon` など）:

```rust
// .lock() → .write() に変更
let mut w = world.0.write().unwrap();
```

読み取り専用 NIF（`get_enemy_count`, `get_hud_data`, `get_frame_time_ms` など）:

```rust
// .lock() → .read() に変更（共有ロック — 複数プロセスが同時に読める）
let w = world.0.read().unwrap();
```

> **注意**: `drain_frame_events` はイベントを取り出してクリアするため  
> `.write()` を使うこと。

### 確認ポイント

- [ ] MagicWand / Fireball / Lightning が正常に動作する
- [ ] 10,000 体時の `physics_ms` が改善前より低下している
- [ ] `StressMonitor` と `GameEvents` が同時に NIF を呼んでもデッドロックしない

---

## 1.3.5 Telemetry 計測基盤

**PRIORITY_STEPS**: P7

### 目標

`:telemetry` を導入し、ゲームループの各フェーズを**標準的な方法で計測**できるようにする。  
将来的に Phoenix LiveDashboard や Prometheus と接続できる基盤を整える。

### なぜ重要か

`StressMonitor` は独自ログで性能を監視しているが、  
`:telemetry` を使えば Elixir エコシステムの標準ツールと連携できる。  
計測コードとゲームロジックを完全に分離できる。

### 実装内容

#### 30.1 `mix.exs` に依存関係を追加

```elixir
# mix.exs
defp deps do
  [
    {:rustler, "~> 0.34"},
    {:telemetry,         "~> 1.3"},
    {:telemetry_metrics, "~> 1.0"},
  ]
end
```

```powershell
mix deps.get
```

#### 30.2 `Game.Telemetry` Supervisor を新規作成

```elixir
# lib/game/telemetry.ex（新規作成）
defmodule Game.Telemetry do
  @moduledoc """
  Telemetry イベントのハンドラーと Metrics を定義する Supervisor。

  計測ポイント:
    [:game, :tick]          — 毎フレームの物理ステップ時間・敵数
    [:game, :level_up]      — レベルアップ発生
    [:game, :boss_spawn]    — ボス出現
    [:game, :session_end]   — ゲームオーバー

  将来的な拡張:
    - Phoenix LiveDashboard との接続
    - Prometheus / Grafana へのエクスポート
    - ゲームセッションのリプレイ記録
  """

  use Supervisor

  def start_link(opts), do: Supervisor.start_link(__MODULE__, opts, name: __MODULE__)

  @impl true
  def init(_opts) do
    children = [
      {Telemetry.Metrics.ConsoleReporter, metrics: metrics()},
    ]
    Supervisor.init(children, strategy: :one_for_one)
  end

  def metrics do
    [
      Telemetry.Metrics.summary("game.tick.physics_ms",
        unit: {:native, :millisecond},
        description: "Rust physics step duration per frame"
      ),
      Telemetry.Metrics.last_value("game.tick.enemy_count",
        description: "Active enemy count"
      ),
      Telemetry.Metrics.counter("game.level_up.count",
        description: "Total level-up events"
      ),
      Telemetry.Metrics.counter("game.boss_spawn.count",
        description: "Total boss spawn events"
      ),
    ]
  end
end
```

#### 30.3 `GameEvents` に計測ポイントを追加

```elixir
# lib/game/game_loop.ex

# 毎秒ログの直後に追加
if rem(state.frame_count, 60) == 0 do
  # ... 既存のログ処理 ...

  :telemetry.execute(
    [:game, :tick],
    %{physics_ms: physics_ms, enemy_count: enemy_count},
    %{phase: state.phase, wave: wave}
  )
end

# レベルアップ検知時に追加
if level_up_pending and state.phase == :playing do
  :telemetry.execute(
    [:game, :level_up],
    %{level: level},
    %{}
  )
  # ...
end

# ボス出現時に追加（:boss_alert フェーズ遷移時）
:telemetry.execute(
  [:game, :boss_spawn],
  %{},
  %{boss: state.pending_boss_name}
)

# ゲームオーバー時に追加
:telemetry.execute(
  [:game, :session_end],
  %{elapsed_seconds: elapsed / 1000.0},
  %{score: score}
)
```

#### 30.4 `Application` に `Game.Telemetry` を追加

```elixir
# lib/game/application.ex

children = [
  Game.InputHandler,
  Game.EventBus,
  Game.GameEvents,
  Game.StressMonitor,
  Game.Stats,
  Game.Telemetry,     # ← 追加
]
```

#### 30.5 動作確認: IEx から計測データを確認

```elixir
# IEx でハンドラーを手動アタッチして確認
:telemetry.attach(
  "debug-handler",
  [:game, :tick],
  fn event, measurements, metadata, _config ->
    IO.inspect({event, measurements, metadata}, label: "telemetry")
  end,
  nil
)
```

### 確認ポイント

- [ ] `mix deps.get` が成功する
- [ ] ゲーム起動後、コンソールに Telemetry のサマリーが出力される
- [ ] `iex> :telemetry.list_handlers([:game, :tick])` でハンドラーが確認できる
- [ ] ゲームオーバー時に `[:game, :session_end]` イベントが発火する

---

## 1.3.6 SIMD AI 高速化（オプション）

**PRIORITY_STEPS**: P4（オプション）

### 目標

`update_chase_ai` の Chase AI 計算を SIMD（SSE2）で 4 要素同時処理し、  
rayon による並列化と組み合わせて最大限の性能を引き出す。

### なぜ重要か

`update_chase_ai` は rayon で並列化済みだが、各要素の計算（正規化・乗算）は  
スカラー演算。SSE2 SIMD を使えば 1 命令で 4 要素を同時処理できる。

> **前提条件**: x86_64 アーキテクチャ（Windows / Linux / macOS Intel）。  
> ARM（Apple Silicon 等）では `std::arch::aarch64` の NEON 命令を使う。  
> フォールバックとして既存の rayon 版を必ず残すこと。

### 実装内容

#### 31.1 SIMD 版 Chase AI を追加

```rust
// native/game_native/src/lib.rs

/// SIMD（SSE2）版 Chase AI — x86_64 専用
/// rayon 版と同じ結果を返すが、4 要素を同時処理する
#[cfg(target_arch = "x86_64")]
pub fn update_chase_ai_simd(
    enemies: &mut EnemyWorld,
    player_x: f32, player_y: f32,
    dt: f32,
) {
    use std::arch::x86_64::*;

    let len = enemies.len();
    // SIMD で処理できる 4 の倍数部分
    let simd_len = (len / 4) * 4;

    unsafe {
        let px4  = _mm_set1_ps(player_x);
        let py4  = _mm_set1_ps(player_y);
        let dt4  = _mm_set1_ps(dt);
        let eps4 = _mm_set1_ps(0.001_f32);

        for base in (0..simd_len).step_by(4) {
            // alive チェック（4 体すべて生存の場合のみ SIMD 処理）
            if !enemies.alive[base]   || !enemies.alive[base+1]
            || !enemies.alive[base+2] || !enemies.alive[base+3] {
                // 混在する場合はスカラーにフォールバック
                for i in base..base+4 {
                    if i < len && enemies.alive[i] {
                        scalar_chase_one(enemies, i, player_x, player_y, dt);
                    }
                }
                continue;
            }

            // 4 要素を同時ロード
            let ex = _mm_loadu_ps(enemies.positions_x[base..].as_ptr());
            let ey = _mm_loadu_ps(enemies.positions_y[base..].as_ptr());
            let sp = _mm_loadu_ps(enemies.speeds[base..].as_ptr());

            // 方向ベクトルを計算
            let dx = _mm_sub_ps(px4, ex);
            let dy = _mm_sub_ps(py4, ey);

            // 距離の二乗
            let dist_sq = _mm_add_ps(_mm_mul_ps(dx, dx), _mm_mul_ps(dy, dy));

            // 逆平方根（高速近似）— max(eps) でゼロ除算を防ぐ
            let dist_sq_safe = _mm_max_ps(dist_sq, eps4);
            let inv_dist = _mm_rsqrt_ps(dist_sq_safe);

            // 速度を計算
            let vx = _mm_mul_ps(_mm_mul_ps(dx, inv_dist), sp);
            let vy = _mm_mul_ps(_mm_mul_ps(dy, inv_dist), sp);

            // 位置を更新
            let new_ex = _mm_add_ps(ex, _mm_mul_ps(vx, dt4));
            let new_ey = _mm_add_ps(ey, _mm_mul_ps(vy, dt4));

            // 書き戻し
            _mm_storeu_ps(enemies.positions_x[base..].as_mut_ptr(), new_ex);
            _mm_storeu_ps(enemies.positions_y[base..].as_mut_ptr(), new_ey);
            _mm_storeu_ps(enemies.velocities_x[base..].as_mut_ptr(), vx);
            _mm_storeu_ps(enemies.velocities_y[base..].as_mut_ptr(), vy);
        }

        // 残り要素をスカラーで処理
        for i in simd_len..len {
            if enemies.alive[i] {
                scalar_chase_one(enemies, i, player_x, player_y, dt);
            }
        }
    }
}

#[inline]
fn scalar_chase_one(
    enemies: &mut EnemyWorld,
    i: usize,
    player_x: f32, player_y: f32,
    dt: f32,
) {
    let dx   = player_x - enemies.positions_x[i];
    let dy   = player_y - enemies.positions_y[i];
    let dist = (dx * dx + dy * dy).sqrt().max(0.001);
    let speed = enemies.speeds[i];
    enemies.velocities_x[i] = (dx / dist) * speed;
    enemies.velocities_y[i] = (dy / dist) * speed;
    enemies.positions_x[i] += enemies.velocities_x[i] * dt;
    enemies.positions_y[i] += enemies.velocities_y[i] * dt;
}
```

#### 31.2 `physics_step` 内で切り替える

```rust
// physics_step 内の update_chase_ai 呼び出しを条件コンパイルで切り替え

#[cfg(target_arch = "x86_64")]
update_chase_ai_simd(&mut w.enemies, px, py, dt);

#[cfg(not(target_arch = "x86_64"))]
update_chase_ai(&mut w.enemies, px, py, dt);
```

#### 31.3 ベンチマークで効果を確認

```rust
// native/game_native/benches/ai_bench.rs（新規作成）
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_chase_ai(c: &mut Criterion) {
    let mut enemies = setup_enemies(10_000);
    c.bench_function("chase_ai_scalar_rayon", |b| {
        b.iter(|| update_chase_ai(&mut enemies, 640.0, 360.0, 0.016))
    });

    #[cfg(target_arch = "x86_64")]
    c.bench_function("chase_ai_simd", |b| {
        b.iter(|| update_chase_ai_simd(&mut enemies, 640.0, 360.0, 0.016))
    });
}

criterion_group!(benches, bench_chase_ai);
criterion_main!(benches);
```

```powershell
cargo bench --bench ai_bench
```

### 確認ポイント

- [ ] `cargo build --release` が成功する
- [ ] ゲームが正常に動作する（敵の動きに変化がない）
- [ ] ベンチマークで SIMD 版が rayon 版より高速（目安: 2〜4 倍）
- [ ] ARM 環境（Apple Silicon 等）でもビルドが通る（フォールバックが機能する）

---

## 改善効果まとめ

推奨実施順序は [PRIORITY_STEPS.md](../04_roadmap/PRIORITY_STEPS.md) を参照。

| ステップ | 改善内容 | 期待効果 | Elixir の真価 |
|---|---|---|---|
| **1.3.4** | Spatial Hash 最近接 + RwLock | AI 探索 O(N) → O(数十)・ロック競合解消 | — Rust 側改善 |
| **1.3.3** | フリーリスト | スポーン O(N) → O(1) | — Rust 側改善 |
| **1.3.1** | イベントバス | Stats が機能する・拡張性向上 | ◎ OTP の関心分離 |
| **1.3.2** | ETS キャッシュ + 入力ポーリング | NIF 呼び出し削減・メッセージキュー軽減 | ◎ ETS 活用 |
| **1.3.5** | Telemetry | 標準的な計測基盤・LiveDashboard 対応 | ◎ 観測可能性 |
| **1.3.6** | SIMD | Chase AI 2〜4 倍高速化 | — Rust 上級 |

---

## 完成後のアーキテクチャ

```
┌─────────────────────────────────────────────────────────┐
│  BEAM VM (Elixir / OTP)                                 │
│                                                         │
│  Game.GameEvents ─── 60Hz tick ──→ NIF (Rust)             │
│       │ ETS 書き込み（毎秒）                              │
│       │ EventBus.broadcast（毎フレーム）                  │
│       │ Telemetry.execute（毎秒）                         │
│       ↓                                                 │
│  :frame_cache (ETS) ←── StressMonitor（ロックフリー読取）│
│  :input_state (ETS) ←── InputHandler（ロックフリー書込） │
│                                                         │
│  Game.EventBus ──→ Game.Stats（イベント受信）            │
│                ──→ 将来: リプレイ記録・実績システム       │
│                                                         │
│  Game.Telemetry ──→ ConsoleReporter                     │
│                 ──→ 将来: LiveDashboard / Prometheus     │
└─────────────────────────────────────────────────────────┘
         ↕ RwLock<GameWorldInner>（読み取り競合なし）
┌─────────────────────────────────────────────────────────┐
│  Rust NIF (game_native)                                 │
│  - physics_step: rayon 並列 AI + SIMD（1.3.6）        │
│  - SoA + フリーリスト（O(1) スポーン）                   │
│  - Spatial Hash（衝突判定 + 最近接探索）                 │
│  - frame_events バッファ（毎フレーム drain）             │
└─────────────────────────────────────────────────────────┘
```

---

## 関連ドキュメント

- [STEPS_ALL.md](./STEPS_ALL.md) — 全体ロードマップ・章・節・項構成
- [STEPS_PERFORMANCE_ANALYSIS.md](./STEPS_PERFORMANCE_ANALYSIS.md) — パフォーマンス課題の分析・提案
- [PRIORITY_STEPS.md](../04_roadmap/PRIORITY_STEPS.md) — 実施優先度（P1〜P7）
