# ファイル単位アーキテクチャ（現状）

図解版（Mermaid）: `docs/06_system_design/ARCHITECTURE_FILE_LEVEL_DIAGRAMS.md`

最終更新: 2026-02-25

本ドキュメントは、以下ディレクトリの**現状の実装依存**をファイル単位で整理したもの。

- `lib/app`
- `lib/engine`
- `lib/games`
- `native/game_core/src`
- `native/game_native/src`

---

## 1. 全体像（Elixir <-> Rust 境界）

```text
Elixir層
  lib/app/application.ex
    -> Engine系プロセス起動（SceneManager, GameEvents管理, EventBus, Telemetry等）
  lib/app/nif_bridge.ex
    <-> Rustlerで native/game_native をロード
    <-> Elixir.App.NifBridge としてNIF API公開

Rust層
  native/game_native/src/lib.rs
    -> rustler::init!("Elixir.App.NifBridge", load = nif::load)
    -> nif/* が Elixir公開関数を実装
    -> world/* + game_logic/* + renderer/* + render_thread.rs
    -> game_core を利用

共通コア層
  native/game_core/src/*
    -> 定数、パラメータテーブル、物理プリミティブ、ユーティリティ
```

---

## 2. `lib/app`（ファイル単位）

```text
lib/app/application.ex
  -> Registry(name: Engine.RoomRegistry)
  -> Engine.SceneManager
  -> Engine.InputHandler
  -> Engine.EventBus
  -> Engine.RoomSupervisor
  -> Engine.StressMonitor
  -> Engine.Stats
  -> Engine.Telemetry
  -> Engine.RoomSupervisor.start_room(:main)
  -> 環境変数 GAME_ASSETS_ID 設定

lib/app/nif_bridge.ex
  -> use Rustler, crate: :game_native
  -> NIF公開関数群（create_world, physics_step, start_render_thread等）
  <- native/game_native/src/lib.rs の rustler::init と接続
```

---

## 3. `lib/engine`（ファイル単位）

```text
lib/engine/game.ex
  -> Engine.Game behaviour定義（ゲーム実装契約）

lib/engine/scene_behaviour.ex
  -> Engine.SceneBehaviour定義（シーン実装契約）

lib/engine/scene_manager.ex
  -> configの :game, :current からゲームモジュール取得
  -> game.initial_scenes() をシーンスタック化

lib/engine/game_events.ex  [中心]
  -> Engine.create_world / set_map_obstacles / create_game_loop_control
  -> Engine.start_rust_game_loop / start_render_thread
  -> Engine.SceneManager (current/push/pop/replace/update)
  -> Engine.EventBus.broadcast
  -> Engine.FrameCache.put
  -> Engine.InputHandler.get_move_vector（非main）
  -> Engine.SaveManager系API（Engine経由）
  -> App.NifBridge.get_frame_metadata / get_weapon_levels（直接参照あり）

lib/engine/event_bus.ex
  -> subscribeされたプロセスへ {:game_events, events} を配信

lib/engine/frame_cache.ex
  <- GameEventsが更新
  -> StressMonitorが参照

lib/engine/input_handler.ex
  -> ETS :input_state を管理
  <- GameEventsが移動ベクトル取得

lib/engine/map_loader.ex
  <- GameEvents.init で障害物ロード

lib/engine/room_registry.ex
  <-> Registry API（room_id -> pid）

lib/engine/room_supervisor.ex
  -> DynamicSupervisor
  -> Engine.GameEvents をルーム単位で起動停止
  -> Engine.RoomRegistry 参照

lib/engine/save_manager.ex
  -> App.NifBridge.get_save_snapshot / load_save_snapshot
  -> ファイル保存（saves/*）

lib/engine/stats.ex
  -> Engine.EventBus.subscribe
  <- EventBusのイベントを集計

lib/engine/stress_monitor.ex
  -> Engine.FrameCache.get
  -> パフォーマンスログ出力

lib/engine/telemetry.ex
  -> Telemetry.Metrics.ConsoleReporter
```

補足: `lib/engine.ex` は公開APIファサードとして `App.NifBridge` / `SceneManager` / `SaveManager` / `RoomSupervisor` を仲介する。

---

## 4. `lib/games`（ファイル単位）

### 4.1 `mini_shooter`

```text
lib/games/mini_shooter/game.ex
  -> @behaviour Engine.Game
  -> initial_scenes: Playing
  -> physics_scenes: Playing

lib/games/mini_shooter/spawn_system.ex
  -> Engine.get_enemy_count
  -> Engine.spawn_enemies

lib/games/mini_shooter/scenes/playing.ex
  -> Engine.is_player_dead?
  -> SpawnSystem.maybe_spawn
  -> transition: GameOverへreplace

lib/games/mini_shooter/scenes/game_over.ex
  -> ゲームオーバー状態維持
```

### 4.2 `vampire_survivor`

```text
lib/games/vampire_survivor/game.ex
  -> @behaviour Engine.Game
  -> シーン参照（Playing/LevelUp/BossAlert/GameOver）
  -> SpawnSystem.wave_label
  -> LevelSystem.weapon_label

lib/games/vampire_survivor/spawn_system.ex
  -> Engine.get_enemy_count
  -> Engine.spawn_enemies / spawn_elite_enemy

lib/games/vampire_survivor/level_system.ex
  -> 武器候補生成（純関数）

lib/games/vampire_survivor/boss_system.ex
  -> ボス出現判定（純関数）

lib/games/vampire_survivor/scenes/playing.ex
  -> Engine.is_player_dead?
  -> BossSystem.check_spawn
  -> LevelSystem.generate_weapon_choices
  -> SpawnSystem.maybe_spawn
  -> Engine.get_level_up_data / skip_level_up
  -> transition: push/replace

lib/games/vampire_survivor/scenes/level_up.ex
  -> 一定時間で :pop（auto_select）

lib/games/vampire_survivor/scenes/boss_alert.ex
  -> BossSystem.alert_duration_ms
  -> Engine.spawn_boss

lib/games/vampire_survivor/scenes/game_over.ex
  -> ゲームオーバー状態維持
```

---

## 5. `native/game_core/src`（ファイル単位）

```text
native/game_core/src/lib.rs
  -> boss/constants/enemy/entity_params/item/physics/util/weapon を公開

native/game_core/src/constants.rs
  -> 画面・物理・戦闘定数

native/game_core/src/entity_params.rs
  -> Enemy/Weapon/Boss のIDベーステーブル（現行主要経路）

native/game_core/src/item.rs
  -> ItemKind + ItemWorld(SoA)

native/game_core/src/weapon.rs
  -> WeaponSlot + weapon_upgrade_desc
  -> constants/entity_params 参照

native/game_core/src/util.rs
  -> exp_required_for_next / spawn_position_around_player

native/game_core/src/physics/mod.rs
  -> obstacle_resolve / rng / separation / spatial_hash

native/game_core/src/physics/rng.rs
  -> SimpleRng

native/game_core/src/physics/spatial_hash.rs
  -> SpatialHash + CollisionWorld（動的/静的）

native/game_core/src/physics/separation.rs
  -> EnemySeparation trait + apply_separation

native/game_core/src/physics/obstacle_resolve.rs
  -> resolve_obstacles_player

native/game_core/src/enemy.rs, boss.rs
  -> Enum中心の旧来定義（entity_paramsとの併存）
```

---

## 6. `native/game_native/src`（ファイル単位）

### 6.1 入口・NIF公開

```text
native/game_native/src/lib.rs
  -> mod asset/audio/game_logic/nif/renderer/render_snapshot/render_thread/world
  -> rustler::init!("Elixir.App.NifBridge", load = nif::load)
  -> atoms定義（ok, frame_events, ui_action, enemy_killed等）

native/game_native/src/nif/mod.rs
  -> action_nif / game_loop_nif / load / render_nif / read_nif / save_nif / util / world_nif

native/game_native/src/nif/load.rs
  -> Resource登録(GameWorld, GameLoopControl)
  -> アトム事前登録

native/game_native/src/nif/world_nif.rs
  -> create_world / set_player_input / spawn_enemies / set_map_obstacles

native/game_native/src/nif/game_loop_nif.rs
  -> physics_step_inner + drain_frame_events_inner
  -> start_rust_game_loop（スレッド）
  -> pause_physics / resume_physics

native/game_native/src/nif/action_nif.rs
  -> add_weapon / skip_level_up / spawn_boss / spawn_elite_enemy

native/game_native/src/nif/read_nif.rs
  -> get_frame_metadata / get_level_up_data / get_weapon_levels ほか read API

native/game_native/src/nif/save_nif.rs
  -> SaveSnapshot定義
  -> get_save_snapshot / load_save_snapshot

native/game_native/src/nif/render_nif.rs
  -> render_thread::run_render_thread 起動（単一起動ガード）
```

### 6.2 ワールドモデル

```text
native/game_native/src/world/mod.rs
  -> boss/bullet/enemy/frame_event/game_loop_control/game_world/particle/player

native/game_native/src/world/game_world.rs
  -> GameWorldInner（全状態）
  -> complete_level_up / rebuild_collision

native/game_native/src/world/player.rs
  -> PlayerState

native/game_native/src/world/enemy.rs
  -> EnemyWorld(SoA) + EnemySeparation実装

native/game_native/src/world/bullet.rs
  -> BulletWorld(SoA) + BULLET_KIND_* 定数

native/game_native/src/world/particle.rs
  -> ParticleWorld(SoA)

native/game_native/src/world/boss.rs
  -> BossState

native/game_native/src/world/frame_event.rs
  -> FrameEvent enum

native/game_native/src/world/game_loop_control.rs
  -> pause/resume制御
```

### 6.3 ロジック・描画

```text
native/game_native/src/game_logic/mod.rs
  -> chase_ai / events / physics_step

native/game_native/src/game_logic/chase_ai.rs
  -> 追尾AI + 最近接探索 + SIMD/並列処理

native/game_native/src/game_logic/events.rs
  -> FrameEvent -> Elixir送信用Tuple変換

native/game_native/src/game_logic/physics_step.rs  [中心]
  -> world更新（移動、衝突、武器、ボス、ドロップ、イベント）
  -> game_core::constants/entity_params/item/physics/util を大量利用

native/game_native/src/render_snapshot.rs
  -> GameWorldInner -> RenderSnapshot 変換

native/game_native/src/render_thread.rs
  -> winit EventLoop
  -> build_render_snapshot
  -> renderer.update_instances/render
  -> UIアクションをworldへ反映

native/game_native/src/renderer/mod.rs
  -> wgpu描画 + egui HUD/UI
  -> sprite.wgsl 利用

native/game_native/src/renderer/shaders/sprite.wgsl
  -> インスタンススプライト描画シェーダ

native/game_native/src/asset/mod.rs
  -> AssetLoader（実ファイル優先 + 埋め込みフォールバック）

native/game_native/src/audio.rs
  -> rodioによるBGM/SE再生
```

---

## 7. アーキテクチャ評価（強み / 弱み）

### 7.1 強み

1. **層分離が明確**  
   Elixirは運用・遷移・監視、Rustは高頻度計算と描画に集中している。

2. **NIF境界が明確**  
   `lib/app/nif_bridge.ex` と `native/game_native/src/lib.rs` の対応が分かりやすい。

3. **OTP活用が適切**  
   `GameEvents`、`EventBus`、`RoomSupervisor`、`Stats`、`StressMonitor` が分離され、障害局所化しやすい。

4. **データ指向で性能に強い**  
   `EnemyWorld` / `BulletWorld` / `ParticleWorld` が SoA + free list で高負荷に耐えやすい。

5. **ゲーム差し替え可能性**  
   `Engine.Game` / `Engine.SceneBehaviour` によってゲーム固有実装を差し替えられる。

### 7.2 弱み

1. **`physics_step.rs` の責務集中**  
   単一ファイルに複数責務が集まり、保守と安全な変更が難しい。

2. **ドメイン定義の二重化**  
   `game_core` 内で `enemy.rs`/`boss.rs` と `entity_params.rs` が併存し、将来の不整合リスクがある。

3. **境界一貫性の揺れ**  
   一部で `Engine` ファサードを通さず `App.NifBridge` 直接参照があり、抽象層の統一が崩れやすい。

4. **文字列ベースUIアクション**  
   `"__save__"` 等の文字列プロトコルは型安全性が低い。

5. **`GameWorld` 単一ロック依存**  
   `RwLock` に loop/write と render/read が集中し、将来スケールで競合増の可能性がある。

6. **描画モジュール責務過多**  
   `renderer/mod.rs` がレンダリング基盤とゲームUIを同居させている。

---

## 8. 重要ハブ（変更影響が大きい順）

1. `native/game_native/src/game_logic/physics_step.rs`
2. `lib/engine/game_events.ex`
3. `native/game_native/src/world/game_world.rs`
4. `native/game_native/src/renderer/mod.rs`
5. `native/game_native/src/render_thread.rs`
6. `native/game_native/src/nif/read_nif.rs`
7. `native/game_native/src/nif/world_nif.rs`
8. `lib/engine/scene_manager.ex`
9. `native/game_core/src/entity_params.rs`
10. `lib/engine.ex`

