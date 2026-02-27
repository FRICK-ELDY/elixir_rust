# 1.10 方針決定とアーキテクチャ再構築（全10項）

**目的**: Elixir SSOT + Push 型同期への切り替え、Umbrella 化によるスケーラブルな Elixir 側の構築、ネットワーク汎用層の分離。  
**前提**: 1.9（アーキテクチャ改善）完了後に着手する。  
**参照 ADR**: [ADR_SHARED_MEMORY_THREAD_POLICY.md](../../03_tech_decisions/ADR_SHARED_MEMORY_THREAD_POLICY.md)  
**参照設計**: [ELIXIR_RUST_DIVISION.md](../../03_tech_decisions/ELIXIR_RUST_DIVISION.md)

---

## 目標アーキテクチャ

```
elixir_rust/                    ← Umbrella ルート
  mix.exs                       ← Umbrella 定義（apps: [...] のみ）
  apps/
    game_engine/                ← エンジンコア（SSOT・NIF・tick_hz）
    game_content/               ← ゲーム別コンテンツ（VampireSurvivor 等）
    game_network/               ← ネットワーク汎用層（Phoenix・認証・Presence）
    game_server/                ← 本番デプロイ用エントリ（ヘッドレス）
  native/                       ← Rust クレート群（変更なし）
    game_native/
    game_core/
    game_render/
    game_window/

[ローカル起動]
  game_engine + game_content
  └─ Rust NIF（計算・描画・音）をロード

[サーバーデプロイ]
  game_engine + game_content + game_network + game_server
  └─ Rust NIF をロードしない（headless: true）
  └─ Phoenix で WebSocket を受け付ける
```

---

## 1.10.1 設計方針の確定と ADR 更新

**目標**: Elixir SSOT・Push 型同期・tick_hz 可変の方針を ADR に記録する。  
**ステータス**: **完了済み**（2026-02-27）

完了した作業:
- `ADR_SHARED_MEMORY_THREAD_POLICY.md` を Elixir SSOT + Push 型同期 + スレッドポリシーに全面改訂
- `ELIXIR_RUST_DIVISION.md` の役割分担・設計原則を新方針に更新
- `tick_hz` を `10 / 20 / 30Hz` から選択可能な設定キーとして定義

---

## 1.10.2 Umbrella プロジェクト化

**目標**: 現 `:game` 単一アプリを Umbrella 構成に移行し、`apps/game_engine` へ移動する。

### 手順

**Step 1**: Umbrella ルートの `mix.exs` を作成する。

```elixir
# mix.exs（Umbrella ルート）
defmodule GameUmbrella.MixProject do
  use Mix.Project

  def project do
    [
      apps_path: "apps",
      version: "0.1.0",
      elixir: "~> 1.19",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  defp deps do
    []
  end
end
```

**Step 2**: `apps/game_engine/` ディレクトリを作成し、現在の `lib/`・`mix.exs`・`config/` を移動する。

```
# 移動マップ
lib/          → apps/game_engine/lib/
config/       → apps/game_engine/config/
mix.exs       → apps/game_engine/mix.exs（app: :game_engine に変更）
```

**Step 3**: `apps/game_engine/mix.exs` を更新する。

```elixir
defmodule GameEngine.MixProject do
  use Mix.Project

  def project do
    [
      app: :game_engine,
      version: "0.1.0",
      build_path: "../../_build",
      config_path: "../../config/config.exs",
      deps_path: "../../deps",
      lockfile: "../../mix.lock",
      elixir: "~> 1.19",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  def application do
    [
      extra_applications: [:logger],
      mod: {GameEngine.Application, []}
    ]
  end

  defp deps do
    [
      {:rustler, "~> 0.34"},
      {:telemetry, "~> 1.3"},
      {:telemetry_metrics, "~> 1.0"}
    ]
  end
end
```

**確認**: `mix compile` が通ること、`iex -S mix` で起動できること。

---

## 1.10.3 `game_engine` アプリ整備

**目標**: `tick_hz` 設定・ヘッドレスモード対応を `game_engine` に組み込む。

### tick_hz の設定化

`GameEngine.Application` の起動時に `tick_hz` を読み込む。

```elixir
# apps/game_engine/lib/game_engine/application.ex
defmodule GameEngine.Application do
  use Application

  @impl true
  def start(_type, _args) do
    tick_hz = Application.get_env(:game_engine, :tick_hz, 20)
    headless = Application.get_env(:game_engine, :headless, false)

    # tick_interval_ms を GameEvents に渡す
    tick_interval_ms = div(1000, tick_hz)

    children = build_children(headless, tick_interval_ms)
    opts = [strategy: :one_for_one, name: GameEngine.Supervisor]
    Supervisor.start_link(children, opts)
  end

  defp build_children(headless, tick_interval_ms) do
    base = [
      {Registry, [keys: :unique, name: Engine.RoomRegistry]},
      Engine.SceneManager,
      Engine.InputHandler,
      Engine.EventBus,
      {Engine.RoomSupervisor, [tick_interval_ms: tick_interval_ms]},
      Engine.StressMonitor,
      Engine.Stats,
      Engine.Telemetry
    ]

    # ヘッドレス時は NIF（描画・音）をロードしない
    if headless do
      base
    else
      [{App.NifBridge, []} | base]
    end
  end
end
```

### config での設定例

```elixir
# config/config.exs
import Config

# デフォルト: 20Hz
config :game_engine, tick_hz: 20, headless: false

# サーバー用（config/prod.exs 等）
# config :game_engine, tick_hz: 20, headless: true
```

---

## 1.10.4 `game_content` アプリ分離

**目標**: `lib/games/` 配下のゲーム別ロジックを `apps/game_content` に分離する。

### 移動マップ

```
apps/game_engine/lib/games/  →  apps/game_content/lib/
  vampire_survivor/               game_content/
  mini_shooter/                     vampire_survivor/
                                    mini_shooter/
```

### `apps/game_content/mix.exs`

```elixir
defmodule GameContent.MixProject do
  use Mix.Project

  def project do
    [
      app: :game_content,
      version: "0.1.0",
      build_path: "../../_build",
      config_path: "../../config/config.exs",
      deps_path: "../../deps",
      lockfile: "../../mix.lock",
      elixir: "~> 1.19",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  defp deps do
    [
      {:game_engine, in_umbrella: true}
    ]
  end
end
```

**境界ルール**:
- `game_content` は `game_engine` にのみ依存する。
- `game_engine` は `game_content` を知らない（依存逆転禁止）。
- ゲームの切り替えは `game_engine` の設定（`:current_game`）で行う。

---

## 1.10.5 Push 型同期 NIF の実装

**目標**: 旧 `physics_step` NIF を Push 型（`push_tick` / `physics_result`）に置き換える。

### NIF API 変更

| 旧 API | 新 API | 区分 |
|--------|--------|------|
| `physics_step(world, input)` → `{:ok, frame_events}` | `push_tick(world, inputs, commands)` → `{:ok, physics_result}` | `control` |

- `inputs`: プレイヤー入力・移動方向など（軽量・毎 tick 送る）
- `commands`: `spawn_enemy` / `pause` / `resume` など制御コマンド（必要時のみ）
- `physics_result`: 変化したエンティティの delta のみ（全量ではない）

### Elixir 側の tick フロー（`Engine.GameEvents`）

```elixir
defp handle_tick(state) do
  # 1. ゲームロジック更新
  state = update_game_logic(state)

  # 2. 入力・コマンドを Rust へ push（control NIF）
  #    ※ 全エンティティの状態は送らない。入力と制御コマンドのみ。
  inputs = collect_inputs(state)
  commands = collect_commands(state)
  {:ok, delta} = Engine.Commands.push_tick(state.world, inputs, commands)

  # 3. SSOT に delta を適用して更新
  state = apply_delta(state, delta)

  # 4. 次の tick をスケジュール
  Process.send_after(self(), :tick, state.tick_interval_ms)
  state
end
```

### Rust 側の NIF 実装（`game_native`）

```rust
// push_tick: Elixir から入力・コマンドを受け取り、物理計算して delta を返す
// ※ 全状態の転送ではなく、入力と制御コマンドのみを受け取る
#[rustler::nif]
fn push_tick(
    world: ResourceArc<RwLock<GameWorldInner>>,
    inputs: InputsTerm,
    commands: CommandsTerm,
) -> NifResult<DeltaTerm> {
    let mut w = world.write().map_err(|_| rustler::Error::Term(...))?;
    // 入力・コマンドを適用
    w.apply_inputs(inputs);
    w.apply_commands(commands);
    // 物理計算を 1 tick 分実行し、変化分（delta）のみを返す
    let delta = w.run_physics_step();
    Ok(delta.into_term())
}
```

---

## 1.10.6 Rust 計算・描画・音スレッドの整理

**目標**: 3 スレッドの駆動モデルを明確に分離し、`tick_hz` の変化に対して安定した動作を保証する。

### 駆動モデルの違い

3 スレッドは **駆動方式が異なる**。計算スレッドだけは Elixir の push に応じて動き、描画・音は独立した 60Hz ループで動く。

| スレッド | 駆動方式 | 実行タイミング |
|----------|---------|---------------|
| 計算スレッド | **Push 駆動**（Elixir 起点） | `push_tick` NIF が呼ばれるたびに 1 回実行（`tick_hz` に従う） |
| 描画スレッド | **自律ループ**（60Hz 固定） | 独立した 60Hz ループ。計算スレッドの完了を待たない |
| 音スレッド | **自律ループ**（60Hz 固定） | 独立した 60Hz ループ。コマンドキューを消費 |

> 計算スレッドは「60Hz で回る」のではなく、「Elixir が `push_tick` を呼ぶたびに 1 回物理計算を実行する」。  
> 描画・音は Elixir の tick とは無関係に 60Hz で動き続け、計算結果を非同期に参照する。

### スレッド構成

```rust
// game_native の起動時に spawn
pub fn start_runtime(world: Arc<RwLock<GameWorldInner>>, calc_rx: Receiver<TickInput>) {
    // 計算スレッド: push_tick NIF からチャネル経由で入力を受け取り、都度 1 回実行
    let calc_world = Arc::clone(&world);
    std::thread::spawn(move || {
        for input in calc_rx {          // Elixir の push_tick ごとに 1 回ブロック解除
            let mut w = calc_world.write().unwrap();
            w.apply_inputs(input);
            w.run_physics_step();       // 1 tick 分の物理計算
        }
    });

    // 描画スレッド: 独立した 60Hz ループ（計算スレッドと非同期）
    let render_world = Arc::clone(&world);
    std::thread::spawn(move || render_loop(render_world));

    // 音スレッド: 独立した 60Hz ループ
    let audio_world = Arc::clone(&world);
    std::thread::spawn(move || audio_loop(audio_world));
}
```

### 各スレッドの役割

| スレッド | 駆動方式 | 状態への関与 |
|----------|---------|-------------|
| 計算スレッド | Push 駆動（`push_tick` ごとに 1 回） | 入力・コマンドを適用し物理計算。結果（delta）を world に書き込む |
| 描画スレッド | 自律 60Hz ループ | world を read lock で読み、前後フレームを補間して描画。状態を変更しない |
| 音スレッド | 自律 60Hz ループ | コマンドキューを消費して rodio で再生。world を直接変更しない |

---

## 1.10.7 描画スレッドの補間実装

**目標**: 計算スレッドが `tick_hz`（10〜30Hz）で更新する world の状態を、描画スレッドが 60Hz に補間して滑らかな描画を実現する。

### 補間の仕組み

計算スレッドは `push_tick` のたびに world を更新し、更新時刻（タイムスタンプ）を記録する。  
描画スレッドは独立した 60Hz ループで、直前の 2 つのタイムスタンプ間を補間してフレームを生成する。

```
計算スレッド（push_tick ごとに 1 回 = tick_hz に従う）
  → world に delta を適用し、タイムスタンプ t=1000 を記録

描画スレッド（自律 60Hz = 16.67ms ごと）
  → world を read lock で読み、前回更新（t=950）と今回（t=1000）の間を alpha で補間
  → alpha = (now_ms - prev_t) / (curr_t - prev_t)
  → 補間位置 = prev_pos + (curr_pos - prev_pos) * alpha
```

### Rust 実装イメージ

ロック保持時間を最小化するため、**ロック内では補間に必要なデータのコピーのみ**を行い、  
補間計算はロック解放後に実行する。

```rust
fn render_loop(world: Arc<RwLock<GameWorldInner>>) {
    let target_fps = 60;
    let frame_ms = 1000 / target_fps;

    loop {
        let frame_start = Instant::now();

        // Step 1: ロック内では補間に必要なデータだけを素早くコピーして即解放
        //         build_interpolated_frame のような重い処理はロック内で行わない
        let snapshot = {
            let w = world.read().unwrap();
            w.copy_interpolation_data() // prev_state / curr_state / タイムスタンプのみコピー
            // ここでロック解放
        };

        // Step 2: ロック外で補間計算（重い処理はここで行う）
        let now = Instant::now();
        let frame = build_interpolated_frame(&snapshot, now);

        renderer.render(frame);

        // 60Hz に合わせてスリープ
        let elapsed = frame_start.elapsed().as_millis() as u64;
        if elapsed < frame_ms {
            std::thread::sleep(Duration::from_millis(frame_ms - elapsed));
        }
    }
}
```

> **ロック設計の原則**（ADR §lock 競合メトリクス閾値 参照）  
> - read lock 保持時間の警告閾値は **300µs**。  
> - `copy_interpolation_data` はタイムスタンプと位置データの struct コピーのみとし、この閾値内に収める。  
> - 補間計算・頂点バッファ構築など時間のかかる処理はすべてロック外で実行する。

---

## 1.10.8 `game_network` アプリ新規作成

**目標**: Phoenix Socket / Channel・認証・Presence を `game_engine` から独立した汎用アプリとして作成する。

### `apps/game_network/mix.exs`

```elixir
defmodule GameNetwork.MixProject do
  use Mix.Project

  def project do
    [
      app: :game_network,
      version: "0.1.0",
      build_path: "../../_build",
      config_path: "../../config/config.exs",
      deps_path: "../../deps",
      lockfile: "../../mix.lock",
      elixir: "~> 1.19",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  defp deps do
    [
      {:game_engine, in_umbrella: true},
      {:phoenix, "~> 1.7"},
      {:phoenix_pubsub, "~> 2.1"},
      {:ecto_sql, "~> 3.10"},
      {:postgrex, ">= 0.0.0"}
    ]
  end
end
```

### `game_network` の責務

```
apps/game_network/lib/game_network/
  endpoint.ex          ← Phoenix Endpoint（WebSocket）
  socket.ex            ← Socket 接続・user_id 認証
  channels/
    user_channel.ex    ← メッセージ・通知・フレンドイベント
    room_channel.ex    ← ゲームルーム（Engine.RoomSupervisor に委譲）
    lobby_channel.ex   ← プレゼンス（オンライン一覧）
  presence.ex          ← Phoenix.Presence
  accounts/            ← 認証・ユーザー管理（Ecto）
  friends/             ← フレンド申請・承認（Ecto）
```

**境界ルール**:
- `game_network` は `game_engine` に依存するが、`game_content` を知らない。
- ゲームルームの制御は `Engine.RoomSupervisor` への委譲のみ。
- Phoenix・Ecto の依存は `game_network` に閉じ、`game_engine` に持ち込まない。

---

## 1.10.9 `game_server` アプリ新規作成

**目標**: 本番サーバーデプロイ用のエントリアプリを作成し、ヘッドレス起動設定を集約する。

### `apps/game_server/mix.exs`

```elixir
defmodule GameServer.MixProject do
  use Mix.Project

  def project do
    [
      app: :game_server,
      version: "0.1.0",
      build_path: "../../_build",
      config_path: "../../config/config.exs",
      deps_path: "../../deps",
      lockfile: "../../mix.lock",
      elixir: "~> 1.19",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  def application do
    [
      extra_applications: [:logger],
      mod: {GameServer.Application, []}
    ]
  end

  defp deps do
    [
      {:game_engine, in_umbrella: true},
      {:game_content, in_umbrella: true},
      {:game_network, in_umbrella: true}
    ]
  end
end
```

### サーバー用 config

```elixir
# apps/game_server/config/prod.exs
import Config

# ヘッドレス: Rust スレッド（描画・音）をロードしない
config :game_engine,
  tick_hz: 20,
  headless: true

# Phoenix Endpoint
config :game_network, GameNetwork.Endpoint,
  url: [host: "example.com"],
  http: [port: 4000]
```

### デプロイ構成

```
[ローカルクライアント]
  mix run --no-halt  # game_engine + game_content のみ起動
  → headless: false → Rust NIF ロード

[本番サーバー]
  cd apps/game_server && mix phx.server
  → headless: true → Rust NIF をロードしない
  → Phoenix で WebSocket を受け付ける
```

---

## 1.10.10 動作確認・設計文書更新

**目標**: ローカル起動・サーバー起動の両方で動作確認し、設計文書を更新する。

### 動作確認チェックリスト

**ローカル起動**:
- [ ] `iex -S mix` で Umbrella 全アプリが起動する
- [ ] `tick_hz: 20` で GameEvents が 20Hz で tick する
- [ ] `push_tick` → `physics_result(delta)` の往復が動作する
- [ ] 描画スレッドが 60Hz で補間描画する
- [ ] ゲームが正常にプレイできる

**サーバー起動**:
- [ ] `headless: true` で Rust NIF がロードされない
- [ ] Phoenix Endpoint が起動する
- [ ] WebSocket 接続が受け付けられる
- [ ] `Engine.RoomSupervisor` がルームを管理できる

### 更新が必要なドキュメント

| ドキュメント | 更新内容 |
|---|---|
| `ARCHITECTURE.md` | Umbrella アプリ構成図を追加 |
| `FOLDER_CONNECTIONS.md` | `apps/` 配下のレイヤー図に更新 |
| `docs_index.md` | 最終更新日・新ドキュメントの追記 |

---

## 依存関係まとめ

```
1.10.1（ADR 更新）     → 完了済み
1.10.2（Umbrella 化）  → 1.10.1 完了後
1.10.3（game_engine）  → 1.10.2 完了後
1.10.4（game_content） → 1.10.3 完了後
1.10.5（Push 型 NIF）  → 1.10.3 完了後（1.10.4 と並行可）
1.10.6（Rust スレッド） → 1.10.5 完了後
1.10.7（補間描画）      → 1.10.6 完了後
1.10.8（game_network） → 1.10.3 完了後（1.10.5〜1.10.7 と並行可）
1.10.9（game_server）  → 1.10.8 完了後
1.10.10（確認・文書）   → 全項完了後
```

推奨順序: `1.10.2 → 1.10.3 → 1.10.4 / 1.10.5 → 1.10.6 → 1.10.7 → 1.10.8 → 1.10.9 → 1.10.10`
