# ファイル単位アーキテクチャ図解（Mermaid版）

最終更新: 2026-02-25

対象:

- `lib/app`
- `lib/engine`
- `lib/games`
- `native/game_core/src`
- `native/game_native/src`

---

## 1) 全体構成（Elixir <-> Rust）

```mermaid
flowchart LR
  subgraph ELX[Elixir]
    A1[lib/app/application.ex]
    A2[lib/app/nif_bridge.ex]
    E0[lib/engine.ex]
    E1[lib/engine/game_events.ex]
    E2[lib/engine/scene_manager.ex]
    E3[lib/engine/event_bus.ex]
    E4[lib/engine/frame_cache.ex]
    E5[lib/engine/room_supervisor.ex]
  end

  subgraph RST[Rust native/game_native]
    R0[src/lib.rs rustler::init]
    R1[src/nif/*]
    R2[src/world/*]
    R3[src/game_logic/*]
    R4[src/render_thread.rs]
    R5[src/render_snapshot.rs]
    R6[src/renderer/mod.rs]
  end

  subgraph CORE[shared native/game_core]
    C0[src/constants.rs]
    C1[src/entity_params.rs]
    C2[src/physics/*]
    C3[src/util.rs]
    C4[src/weapon.rs]
    C5[src/item.rs]
  end

  A1 --> E2
  A1 --> E3
  A1 --> E5
  E1 --> E0
  E0 --> A2
  A2 --> R0
  R0 --> R1
  R1 --> R2
  R1 --> R3
  R1 --> R4
  R4 --> R5
  R4 --> R6

  R2 --> C1
  R3 --> C0
  R3 --> C1
  R3 --> C2
  R3 --> C3
  R6 --> C0
  R6 --> C4
  R2 --> C5
```

---

## 2) `lib/app`（ファイル単位）

```mermaid
flowchart TD
  AP[application.ex]
  NB[nif_bridge.ex]

  AP --> RR[Engine.RoomRegistry]
  AP --> SM[Engine.SceneManager]
  AP --> IH[Engine.InputHandler]
  AP --> EB[Engine.EventBus]
  AP --> RS[Engine.RoomSupervisor]
  AP --> ST[Engine.Stats]
  AP --> TM[Engine.Telemetry]
  AP --> MON[Engine.StressMonitor]
  AP --> MAIN[start_room :main]

  NB -->|Rustler crate game_native| RLIB[native/game_native/src/lib.rs]
```

---

## 3) `lib/engine`（ファイル単位）

```mermaid
flowchart TD
  GEV[game_events.ex]
  ENG[lib/engine.ex API facade]
  GSM[scene_manager.ex]
  GSB[scene_behaviour.ex]
  GGB[game.ex behaviour]
  EBUS[event_bus.ex]
  FC[frame_cache.ex]
  IH[input_handler.ex]
  ML[map_loader.ex]
  RR[room_registry.ex]
  RS[room_supervisor.ex]
  SV[save_manager.ex]
  ST[stats.ex]
  SM[stress_monitor.ex]
  TEL[telemetry.ex]

  GEV --> ENG
  GEV --> GSM
  GEV --> EBUS
  GEV --> FC
  GEV --> IH
  GEV --> ML
  GEV --> RR
  GEV --> SV

  RS --> RR
  RS --> GEV

  ST --> EBUS
  SM --> FC

  GSM --> GGB
  GEV --> GGB
  GEV --> GSB
  TEL --> TMR[Telemetry.Metrics.ConsoleReporter]
```

---

## 4) `lib/games`（ファイル単位）

```mermaid
flowchart LR
  subgraph MINI[games/mini_shooter]
    M0[game.ex]
    M1[spawn_system.ex]
    M2[scenes/playing.ex]
    M3[scenes/game_over.ex]
    M0 --> M2
    M0 --> M3
    M2 --> M1
  end

  subgraph VS[games/vampire_survivor]
    V0[game.ex]
    V1[spawn_system.ex]
    V2[level_system.ex]
    V3[boss_system.ex]
    V4[scenes/playing.ex]
    V5[scenes/level_up.ex]
    V6[scenes/boss_alert.ex]
    V7[scenes/game_over.ex]

    V0 --> V4
    V0 --> V5
    V0 --> V6
    V0 --> V7
    V4 --> V1
    V4 --> V2
    V4 --> V3
    V6 --> V3
  end
```

---

## 5) `native/game_core/src`（ファイル単位）

```mermaid
flowchart TD
  L[lib.rs]
  K[constants.rs]
  P[entity_params.rs]
  I[item.rs]
  W[weapon.rs]
  U[util.rs]
  E[enemy.rs]
  B[boss.rs]

  PM[physics/mod.rs]
  PR[rng.rs]
  PSH[spatial_hash.rs]
  PSEP[separation.rs]
  POB[obstacle_resolve.rs]

  L --> K
  L --> P
  L --> I
  L --> W
  L --> U
  L --> E
  L --> B
  L --> PM

  PM --> PR
  PM --> PSH
  PM --> PSEP
  PM --> POB

  W --> K
  W --> P
  U --> K
```

---

## 6) `native/game_native/src`（ファイル単位）

```mermaid
flowchart TD
  L[src/lib.rs]

  subgraph NIF[nif]
    N0[nif/mod.rs]
    N1[nif/load.rs]
    N2[nif/world_nif.rs]
    N3[nif/game_loop_nif.rs]
    N4[nif/action_nif.rs]
    N5[nif/read_nif.rs]
    N6[nif/save_nif.rs]
    N7[nif/render_nif.rs]
    N8[nif/util.rs]
  end

  subgraph WORLD[world]
    W0[world/mod.rs]
    W1[world/game_world.rs]
    W2[world/player.rs]
    W3[world/enemy.rs]
    W4[world/bullet.rs]
    W5[world/particle.rs]
    W6[world/boss.rs]
    W7[world/frame_event.rs]
    W8[world/game_loop_control.rs]
  end

  subgraph LOGIC[game_logic]
    G0[game_logic/mod.rs]
    G1[chase_ai.rs]
    G2[events.rs]
    G3[physics_step.rs]
  end

  subgraph RENDER[render]
    R1[render_thread.rs]
    R2[render_snapshot.rs]
    R3[renderer/mod.rs]
    R4[renderer/shaders/sprite.wgsl]
  end

  A1[asset/mod.rs]
  AU[audio.rs]

  L --> N0
  L --> W0
  L --> G0
  L --> R1
  L --> R2
  L --> R3
  L --> A1
  L --> AU

  N0 --> N1
  N0 --> N2
  N0 --> N3
  N0 --> N4
  N0 --> N5
  N0 --> N6
  N0 --> N7
  N0 --> N8

  N2 --> W1
  N3 --> G3
  N3 --> G2
  N4 --> W1
  N5 --> W1
  N6 --> W1
  N7 --> R1

  W0 --> W1
  W0 --> W2
  W0 --> W3
  W0 --> W4
  W0 --> W5
  W0 --> W6
  W0 --> W7
  W0 --> W8

  G0 --> G1
  G0 --> G2
  G0 --> G3
  G3 --> W1
  G1 --> W3

  R1 --> R2
  R1 --> R3
  R3 --> R4
  R2 --> W1
```

---

## 7) 実行シーケンス（1フレーム）

```mermaid
sequenceDiagram
  participant App as application.ex
  participant GE as game_events.ex
  participant NIF as nif/game_loop_nif.rs
  participant PHY as game_logic/physics_step.rs
  participant EVT as game_logic/events.rs
  participant SCN as scene_manager + scenes/*
  participant REN as render_thread.rs
  participant RS as render_snapshot.rs
  participant RD as renderer/mod.rs

  App->>GE: 起動(:main room)
  GE->>NIF: start_rust_game_loop(world, control, pid)
  loop 60Hz
    NIF->>PHY: physics_step_inner()
    PHY->>EVT: frame_events生成
    NIF-->>GE: {:frame_events, events}
    GE->>SCN: current scene update/transition
  end

  par 描画ループ
    GE->>NIF: start_render_thread(world)
    REN->>RS: build_render_snapshot(world.read)
    REN->>RD: update_instances + render
    RD-->>REN: UI action(optional)
    REN-->>GE: {:ui_action, action}
  end
```

---

## 8) 評価（強み / 弱み）

### 強み

- Elixir（運用・遷移）とRust（高頻度計算・描画）の責務分離が明確。
- NIF境界が `nif_bridge.ex` と `src/lib.rs` で一意に追える。
- OTPプロセス分離（`GameEvents`、`EventBus`、`Stats`、`StressMonitor`）が効いている。
- SoA + free list により `world/*` が高負荷時に安定しやすい。

### 弱み

- `game_logic/physics_step.rs` に責務が集中（変更影響が大きい）。
- `game_core` で `enemy.rs`/`boss.rs` と `entity_params.rs` が併存。
- UI action が文字列プロトコルで型安全性が低い。
- `GameWorld` 単一 `RwLock` に read/write 競合点が集まりやすい。

