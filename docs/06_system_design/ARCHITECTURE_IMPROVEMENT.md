# 改善アーキテクチャ提案

**対象**: `lib/app` `lib/engine` `lib/games` `native/game_core/src` `native/game_native/src` `native/game_render/src` `native/game_window/src`  
**目的**: 責務の明確化、巨大ファイル分割、NIF 境界の整理  
**ステータス**: 提案

---

## 1. ねらい

現状アーキテクチャの強みは維持しつつ、以下を改善する。

- `native/game_native/src/game_logic/physics_step.rs` の責務集中を解消する
- `native/game_render/src/renderer/mod.rs` の肥大化を分割する
- `App.NifBridge` の API 面積を整理し、境界契約を明確化する
- Elixir 側のゲーム実装差し替え性を維持したまま、変更影響範囲を小さくする

---

## 2. 全体改善案

```mermaid
flowchart LR
  subgraph ELIXIR_LAYER
    app_application[app application]
    engine_api[engine api facade]
    game_events[engine game events]
    scene_manager[engine scene manager]
    event_bus[engine event bus]
    room_supervisor[engine room supervisor]
    save_manager[engine save manager]
    game_modules[games adapters]
  end

  subgraph NIF_BOUNDARY
    nif_bridge[app nif bridge]
    contract_v1[nif contract v1]
  end

  subgraph RUST_APP
    nif_entry[nif entry handlers]
    app_services[application services]
    loop_service[game loop service]
    render_service[render service]
    save_service[save service]
  end

  subgraph RUST_DOMAIN
    world_repo[world repository]
    domain_world[world domain model]
    domain_systems[domain systems]
    domain_events[domain events]
  end

  subgraph RUST_RENDER_WINDOW
    game_render[game render crate]
    game_window[game window crate]
  end

  app_application --> game_events
  app_application --> scene_manager
  app_application --> room_supervisor
  app_application --> event_bus
  app_application --> save_manager

  game_modules --> engine_api
  game_events --> engine_api
  engine_api --> nif_bridge
  save_manager --> nif_bridge

  nif_bridge --> contract_v1
  contract_v1 --> nif_entry

  nif_entry --> app_services
  app_services --> world_repo
  app_services --> loop_service
  app_services --> render_service
  app_services --> save_service

  loop_service --> domain_systems
  domain_systems --> world_repo
  domain_systems --> domain_events
  world_repo --> domain_world

  render_service --> world_repo
  render_service --> game_render
  render_service --> game_window

  save_service --> world_repo
```

---

## 3. Rust 側分割案

```mermaid
flowchart TB
  subgraph GAME_NATIVE
    gn_nif[nif module]
    gn_app[app module]
    gn_loop[loop module]
    gn_render[render module]
    gn_save[save module]
    gn_systems[systems module]
    gn_world[world module]
  end

  subgraph GAME_CORE
    gc_constants[constants]
    gc_entity[entity params]
    gc_physics[physics primitives]
    gc_item_weapon[item and weapon]
  end

  subgraph GAME_RENDER
    gr_facade[renderer facade]
    gr_pipeline[pipeline]
    gr_buffers[buffers]
    gr_sprites[sprites atlas]
    gr_ui[hud and ui]
  end

  gn_nif --> gn_app
  gn_app --> gn_loop
  gn_app --> gn_render
  gn_app --> gn_save
  gn_app --> gn_systems
  gn_systems --> gn_world

  gn_systems --> gc_constants
  gn_systems --> gc_entity
  gn_systems --> gc_physics
  gn_systems --> gc_item_weapon

  gn_render --> gn_world
  gn_render --> gr_facade
  gr_facade --> gr_pipeline
  gr_facade --> gr_buffers
  gr_facade --> gr_sprites
  gr_facade --> gr_ui
```

---

## 4. Elixir 側境界整理案

```mermaid
flowchart LR
  games_mod[games modules]
  engine_api[engine facade]
  engine_commands[engine commands]
  engine_queries[engine queries]
  game_events[game events]
  nif_bridge[nif bridge]
  rust_native[rust native]

  games_mod --> engine_api
  game_events --> engine_api

  engine_api --> engine_commands
  engine_api --> engine_queries

  engine_commands --> nif_bridge
  engine_queries --> nif_bridge
  nif_bridge --> rust_native
```

---

## 5. 段階的移行ステップ

1. `game_native` に `systems` 配下を作り、`physics_step.rs` の関数を機能別に移す  
2. `game_render` で `renderer/mod.rs` を facade 化し、内部モジュールへ分割する  
3. Elixir 側に command と query の入口を追加し、`App.NifBridge` 呼び出しを集約する  
4. 既存 API を deprecate して段階置換し、最終的に旧入口を削除する

---

## 6. 追記課題（クローズ）

### 6.1 `game_native` の `world` module 分離

- **課題**: `world` 配下の責務を `state` `events` `control` に明確分離する
- **背景**: `game_logic` と `nif` からの参照が増え、境界が曖昧になりやすい
- **方針**:
  - `world/state`: `game_world.rs` `player.rs` `enemy.rs` `bullet.rs` `boss.rs` `particle.rs`
  - `world/events`: `frame_event.rs`
  - `world/control`: `game_loop_control.rs`
  - 依存方向は `game_logic -> world` を維持し、`world -> game_logic` を作らない
- **期待効果**:
  - `physics_step` 分割時の依存整理が容易になる
  - `nif` 層の read/write 境界を明示しやすくなる
  - テスト対象を小さく分割できる
- **ステータス**: **Closed（方針合意済み）**
- **次アクション**: 実装開始時に「ファイル移動マップ」と「`mod.rs` 更新差分」を作成して着手する

