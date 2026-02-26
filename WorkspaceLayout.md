# Workspace Layout（自動生成）

## elixir:app

| Path | Lines | Status | Summary |
|------|-------|--------|--------|
| [lib/app/application.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/app/application.ex) | 26 | 🟢 | OTP Application 起動・子プロセススーパービジョン |
| [lib/app/nif_bridge.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/app/nif_bridge.ex) | 41 | 🟢 | Rust NIF のラッパーモジュール（Rustler 経由で game_native をロード） |
| [lib/game.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/game.ex) | 2 | ⚪ | Elixir x Rust ヴァンパイアサバイバーライクゲームのルートモジュール |
## elixir:engine

| Path | Lines | Status | Summary |
|------|-------|--------|--------|
| [lib/engine.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/engine.ex) | 263 | 🔴 | ゲームエンジンの安定化された公開 API |
| [lib/engine/event_bus.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/engine/event_bus.ex) | 41 | 🟢 | フレームイベントをサブスクライバーに配信する GenServer |
| [lib/engine/frame_cache.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/engine/frame_cache.ex) | 37 | 🟢 | フレームごとのゲーム状態を ETS に書き込む |
| [lib/engine/game.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/engine/game.ex) | 39 | 🟢 | ゲームがエンジンに提供すべき behaviour インターフェース |
| [lib/engine/game_events.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/engine/game_events.ex) | 393 | 🔴 | フレームイベント受信・フェーズ管理・NIF 呼び出しの GenServer |
| [lib/engine/input_handler.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/engine/input_handler.ex) | 53 | 🟡 | キー入力を ETS に書き込む GenServer |
| [lib/engine/map_loader.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/engine/map_loader.ex) | 32 | 🟢 | マップ ID に応じた障害物リストの提供 |
| [lib/engine/room_registry.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/engine/room_registry.ex) | 40 | 🟢 | ルーム ID → GameEvents pid の Registry |
| [lib/engine/room_supervisor.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/engine/room_supervisor.ex) | 66 | 🟡 | ルーム単位で GameEvents を管理する DynamicSupervisor |
| [lib/engine/save_manager.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/engine/save_manager.ex) | 114 | 🟠 | セーブ・ロード・ハイスコア永続化 |
| [lib/engine/scene_behaviour.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/engine/scene_behaviour.ex) | 32 | 🟢 | シーンコールバック（init/update/render_type）の behaviour 定義 |
| [lib/engine/scene_manager.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/engine/scene_manager.ex) | 95 | 🟡 | シーンスタック管理の GenServer |
| [lib/engine/stats.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/engine/stats.ex) | 104 | 🟠 | ゲームセッション統計を収集する GenServer |
| [lib/engine/stress_monitor.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/engine/stress_monitor.ex) | 65 | 🟡 | パフォーマンスモニタリング・フレーム超過検出の GenServer |
| [lib/engine/telemetry.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/engine/telemetry.ex) | 40 | 🟢 | Telemetry イベントハンドラーと Metrics の Supervisor |
## elixir:games:mini_shooter

| Path | Lines | Status | Summary |
|------|-------|--------|--------|
| [lib/games/mini_shooter/game.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/games/mini_shooter/game.ex) | 37 | 🟢 | ミニマルゲームの Engine.Game 実装（汎用化検証用） |
| [lib/games/mini_shooter/scenes/game_over.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/games/mini_shooter/scenes/game_over.ex) | 11 | 🟢 | ミニ shooter のゲームオーバーシーン |
| [lib/games/mini_shooter/scenes/playing.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/games/mini_shooter/scenes/playing.ex) | 20 | 🟢 | MiniShooter のプレイ中シーン |
| [lib/games/mini_shooter/spawn_system.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/games/mini_shooter/spawn_system.ex) | 16 | 🟢 | ミニマル敵スポーンシステム（スライム固定間隔） |
## elixir:games:vampire_survivor

| Path | Lines | Status | Summary |
|------|-------|--------|--------|
| [lib/games/vampire_survivor/boss_system.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/games/vampire_survivor/boss_system.ex) | 26 | 🟢 | ボス出現スケジュール管理の純粋関数モジュール（ヴァンサバ固有） |
| [lib/games/vampire_survivor/game.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/games/vampire_survivor/game.ex) | 45 | 🟢 | ヴァンサバの Engine.Game 実装（初期シーン・物理対象の提供） |
| [lib/games/vampire_survivor/level_system.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/games/vampire_survivor/level_system.ex) | 31 | 🟢 | レベルアップ・武器選択生成の純粋関数モジュール（ヴァンサバ固有） |
| [lib/games/vampire_survivor/scenes/boss_alert.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/games/vampire_survivor/scenes/boss_alert.ex) | 23 | 🟢 | ボス出現警告シーン |
| [lib/games/vampire_survivor/scenes/game_over.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/games/vampire_survivor/scenes/game_over.ex) | 11 | 🟢 | ヴァンサバのゲームオーバーシーン |
| [lib/games/vampire_survivor/scenes/level_up.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/games/vampire_survivor/scenes/level_up.ex) | 21 | 🟢 | レベルアップ武器選択シーン |
| [lib/games/vampire_survivor/scenes/playing.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/games/vampire_survivor/scenes/playing.ex) | 73 | 🟡 | ヴァンサバのプレイ中シーン（物理・スポーン・ボス/レベルアップチェック） |
| [lib/games/vampire_survivor/spawn_system.ex](https://github.com/FRICK-ELDY/elixir_rust/blob/main/lib/games/vampire_survivor/spawn_system.ex) | 67 | 🟡 | ウェーブベース敵スポーンシステム（ヴァンサバ固有） |
## rust:native

| Path | Lines | Status | Summary |
|------|-------|--------|--------|
| [native/game_core/src/boss.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_core/src/boss.rs) | 72 | 🟡 | ボス種類・HP・行動の共通定義 |
| [native/game_core/src/constants.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_core/src/constants.rs) | 42 | 🟢 | 画面解像度・マップサイズ・物理定数などの定数定義 |
| [native/game_core/src/enemy.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_core/src/enemy.rs) | 129 | 🟠 | 敵種類・HP・速度・スポーンロジックの共通定義 |
| [native/game_core/src/entity_params.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_core/src/entity_params.rs) | 97 | 🟡 | 敵・武器・ボスの ID ベースパラメータテーブル |
| [native/game_core/src/item.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_core/src/item.rs) | 65 | 🟡 | アイテム種類・レンダー kind の定義と ItemWorld |
| [native/game_core/src/lib.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_core/src/lib.rs) | 5 | 🟢 | ゲームコア共通ロジック（定数・敵・武器・物理プリミティブ） |
| [native/game_core/src/physics/mod.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_core/src/physics/mod.rs) | 1 | ⚪ | 物理モジュールの再エクスポート（衝突・分離・RNG・空間ハッシュ） |
| [native/game_core/src/physics/obstacle_resolve.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_core/src/physics/obstacle_resolve.rs) | 29 | 🟢 | プレイヤーと障害物の衝突解決・押し出し処理 |
| [native/game_core/src/physics/rng.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_core/src/physics/rng.rs) | 32 | 🟢 | 決定論的 LCG 乱数ジェネレータ（no-std 互換） |
| [native/game_core/src/physics/separation.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_core/src/physics/separation.rs) | 67 | 🟡 | 敵同士の重なり解消（Separation）トレイトと適用ロジック |
| [native/game_core/src/physics/spatial_hash.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_core/src/physics/spatial_hash.rs) | 91 | 🟡 | 空間ハッシュによる衝突検出・近傍クエリ |
| [native/game_core/src/util.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_core/src/util.rs) | 70 | 🟡 | 経験値計算・ウェーブ設定・スポーン位置などの共通ユーティリティ |
| [native/game_core/src/weapon.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_core/src/weapon.rs) | 174 | 🟠 | 武器種類・クールダウン・発射ロジックの共通定義 |
| [native/game_native/benches/ai_bench.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/benches/ai_bench.rs) | 42 | 🟢 | Chase AI ベンチマーク（rayon スカラー版 vs SIMD 版） |
| [native/game_native/src/asset/mod.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/asset/mod.rs) | 111 | 🟠 | アセット ID マッピング・実行時ロード・埋め込みフォールバック |
| [native/game_native/src/audio.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/audio.rs) | 40 | 🟢 | BGM・SE 管理（rodio、ループ再生・fire-and-forget） |
| [native/game_native/src/game_logic/chase_ai.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/game_logic/chase_ai.rs) | 193 | 🟠 | 敵 Chase AI と最近接探索（find_nearest_*） |
| [native/game_native/src/game_logic/events.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/game_logic/events.rs) | 16 | 🟢 | フレームイベントの drain（Elixir EventBus 用） |
| [native/game_native/src/game_logic/mod.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/game_logic/mod.rs) | 6 | 🟢 | 物理ステップ・Chase AI・イベント drain |
| [native/game_native/src/game_logic/physics_step.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/game_logic/physics_step.rs) | 777 | 🔴 | 物理ステップ内部実装 |
| [native/game_native/src/lib.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/lib.rs) | 48 | 🟢 | NIF エントリ・モジュール宣言・pub use・rustler::init のみ（スリム化済み） |
| [native/game_native/src/nif/action_nif.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/nif/action_nif.rs) | 57 | 🟡 | アクション NIF（add_weapon, skip_level_up, spawn_boss, spawn_elite_enemy） |
| [native/game_native/src/nif/game_loop_nif.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/nif/game_loop_nif.rs) | 82 | 🟡 | ゲームループ NIF（physics_step, drain_frame_events, pause/resume, Rust ループ起動） |
| [native/game_native/src/nif/load.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/nif/load.rs) | 24 | 🟢 | NIF ローダー（パニックフック・リソース登録・アトム事前登録） |
| [native/game_native/src/nif/mod.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/nif/mod.rs) | 7 | 🟢 | NIF エントリモジュール |
| [native/game_native/src/nif/read_nif.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/nif/read_nif.rs) | 183 | 🟠 | 読み取り専用 NIF（get_*、debug_dump_world、is_player_dead） |
| [native/game_native/src/nif/render_nif.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/nif/render_nif.rs) | 25 | 🟢 | 描画スレッド起動 NIF（1.7.4） |
| [native/game_native/src/nif/save_nif.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/nif/save_nif.rs) | 78 | 🟡 | セーブ・ロード NIF |
| [native/game_native/src/nif/util.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/nif/util.rs) | 1 | ⚪ | NIF 共通ユーティリティ（lock_poisoned_err） |
| [native/game_native/src/nif/world_nif.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/nif/world_nif.rs) | 80 | 🟡 | ワールド作成・入力・スポーン・障害物設定 NIF |
| [native/game_native/src/render_bridge.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/render_bridge.rs) | 64 | 🟡 | game_window の RenderBridge 実装（1.8.4） |
| [native/game_native/src/render_snapshot.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/render_snapshot.rs) | 119 | 🟠 | GameWorld から描画用スナップショットを構築（1.7.5） |
| [native/game_native/src/world/boss.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/world/boss.rs) | 30 | 🟢 | ボス状態（BossState） |
| [native/game_native/src/world/bullet.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/world/bullet.rs) | 79 | 🟡 | 弾丸 SoA（BulletWorld）と描画種別定数 |
| [native/game_native/src/world/enemy.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/world/enemy.rs) | 86 | 🟡 | 敵 SoA（EnemyWorld）と EnemySeparation の実装 |
| [native/game_native/src/world/frame_event.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/world/frame_event.rs) | 5 | 🟢 | フレーム内で発生したゲームイベント（EventBus 用） |
| [native/game_native/src/world/game_loop_control.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/world/game_loop_control.rs) | 16 | 🟢 | GameLoop 制御用（pause/resume）リソース |
| [native/game_native/src/world/game_world.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/world/game_world.rs) | 52 | 🟡 | ゲームワールド（GameWorldInner, GameWorld） |
| [native/game_native/src/world/mod.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/world/mod.rs) | 13 | 🟢 | ワールド型（PlayerState, EnemyWorld, BulletWorld, ParticleWorld, BossState, GameWorld） |
| [native/game_native/src/world/particle.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/world/particle.rs) | 82 | 🟡 | パーティクル SoA（ParticleWorld） |
| [native/game_native/src/world/player.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/world/player.rs) | 5 | 🟢 | プレイヤー状態（座標・入力・HP・無敵タイマー） |
| [native/game_render/src/lib.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_render/src/lib.rs) | 41 | 🟢 | (未設定) |
| [native/game_render/src/renderer/mod.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_render/src/renderer/mod.rs) | 1410 | 🔴 | wgpu によるスプライト描画・パイプライン・テクスチャ管理 |
| [native/game_window/src/lib.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_window/src/lib.rs) | 162 | 🟠 | (未設定) |
## rust:xtask

| Path | Lines | Status | Summary |
|------|-------|--------|--------|
| [native/xtask/src/main.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/xtask/src/main.rs) | 196 | 🟠 | workspace-layout サブコマンドで WorkspaceLayout.md を生成する xtask バイナリ |
