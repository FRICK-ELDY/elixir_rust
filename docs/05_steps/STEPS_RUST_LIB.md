# Rust lib.rs 分割・整理（フォルダ構成検討）

**目的**: Step 48〜54（3D）・Step 55〜61（Slot・コンポーネント）に着手する**前**に、`native/game_native/src/lib.rs` を分割・整理し、保守性と拡張性を高める。  
**前提**: Step 1〜47 の拡張フェーズまで一通り完了していること。2D ゲームの Rust 側が一つの大きな `lib.rs` に集約されている現状を解消する。

**実施タイミング**: 3D・Slot ステップは**据え置き**とし、本「Rust lib 分割・整理」→ **2D ゲームの固め** → [EPIC_ONLINE_SERVICES.md](../06_system_design/EPIC_ONLINE_SERVICES.md) の実装、の順で進める。

---

## 目次

1. [現状の整理](#1-現状の整理)
2. [分割・整理の目標](#2-分割整理の目標)
3. [フォルダ構成案](#3-フォルダ構成案)
4. [実施ステップ（検討項目）](#4-実施ステップ検討項目)
5. [関連ドキュメント](#5-関連ドキュメント)

---

## 1. 現状の整理

### 1.1 現在の `game_native/src` 構成

```
native/game_native/src/
├── lib.rs          # 約 1400 行。NIF・ワールド型・物理ステップ・イベント・セーブ等をすべて含む
├── main.rs         # スタンドアロン描画ループ・ウィンドウ
├── audio.rs        # BGM・SE（rodio）
├── core/           # ゲームロジック共通（main / lib 両方で利用）
│   ├── mod.rs
│   ├── boss.rs, enemy.rs, entity_params.rs, constants.rs, util.rs, weapon.rs, item.rs
│   └── physics/
│       ├── mod.rs, spatial_hash.rs, separation.rs, obstacle_resolve.rs, rng.rs
├── renderer/
│   ├── mod.rs
│   └── shaders/
├── asset/
│   └── mod.rs
```

### 1.2 lib.rs に集約されている主なブロック

| ブロック | 内容 | 行数感 |
|----------|------|--------|
| モジュール・use・atoms | core の利用、rustler の use、atoms! | 先頭〜100 行付近 |
| デバッグ・GameLoopControl | パニックフック、pause/resume 用リソース | 〜66 行 |
| FrameEvent・型定義 | イベント列挙型、PlayerState, EnemyWorld, BulletWorld, ParticleWorld, BossState, GameWorldInner, GameWorld | 〜520 行 |
| ヘルパー・AI | find_nearest_*, scalar_chase_one, update_chase_ai_simd, update_chase_ai | 422〜660 行付近 |
| NIF 群 | create_world, set_player_input, spawn_enemies, physics_step, drain_frame_events, get_* 各種, セーブ/ロード, ゲームループ起動など | 〜1380 行 |
| ローダー | load, rustler::init | 末尾 |

- **課題**: 1 ファイルに「ワールド型定義」「ゲームループ内部ロジック」「NIF エントリ」が混在し、3D や Slot を追加するとさらに肥大化する。

---

## 2. 分割・整理の目標

- **責務の分離**: 「ワールド状態の型」「物理ステップなどのゲームロジック」「NIF のエントリポイント」を別モジュールに分け、lib.rs は `mod` と `pub use` および `rustler::init!` のみに近づける。
- **2D 固め・3D 拡張の土台**: 2D 用ワールドと NIF を整理したうえで、のちに 3D 用モジュールや Slot スナップショット用の型を追加しやすい構成にする。
- **後方互換**: Elixir 側の `App.NifBridge` の NIF 名・引数・戻り値は変えず、Rust 内部のファイル分割のみ行う。

---

## 3. フォルダ構成案

### 案 A: 機能別（nif / world / game_logic）

```
src/
├── lib.rs                 # mod 宣言、pub use、rustler::atoms!、rustler::init!
├── main.rs
├── audio.rs
├── core/                  # 既存のまま（constants, entity_params, physics, weapon, ...）
├── renderer/
├── asset/
├── world/                 # ワールド型と SoA 構造体
│   ├── mod.rs
│   ├── player.rs          # PlayerState
│   ├── enemy.rs           # EnemyWorld (+ EnemySeparation impl)
│   ├── bullet.rs          # BulletWorld
│   ├── particle.rs        # ParticleWorld
│   ├── boss.rs            # BossState（core/boss は BossKind/Params、こちらはランタイム状態）
│   ├── item_world.rs      # ItemWorld（core/item は ItemKind 等、こちらは SoA）
│   └── game_world.rs      # GameWorldInner, GameWorld
├── game_logic/            # 物理ステップとその補助
│   ├── mod.rs
│   ├── physics_step.rs    # physics_step_inner, resolve_obstacles_enemy 等
│   ├── chase_ai.rs        # update_chase_ai, update_chase_ai_simd, find_nearest_*
│   └── events.rs          # FrameEvent, drain_frame_events_inner
└── nif/                   # NIF エントリのみ
    ├── mod.rs
    ├── world_nif.rs       # create_world, set_player_input, spawn_*, get_* 等
    ├── game_loop_nif.rs   # start_rust_game_loop, pause_physics, resume_physics
    └── save_nif.rs        # get_save_snapshot, load_save_snapshot
```

- **利点**: 型（world）・ロジック（game_logic）・NIF（nif）がはっきり分かれる。3D 用に `world3` や `game_logic3` を足しやすい。
- **注意**: `core::boss` と `world::boss` のように名前が被るため、`world::BossState` と `core::BossKind` で役割を区別する。

### 案 B: レイヤー別（state / logic / bridge）

```
src/
├── lib.rs
├── state/                 # 全ワールド状態（Player, Enemy, Bullet, Particle, Item, Boss, GameWorld）
│   └── ...
├── logic/                 # physics_step, chase_ai, events
│   └── ...
├── bridge/                # NIF のみ（Elixir との境界）
│   └── ...
```

- **利点**: 名前が短く「状態 / ロジック / 境界」の 3 層が明確。
- **注意**: 既存の `core` と合わせると「core / state / logic / bridge」の 4 層になり、`core` が「定数・パラメータ・物理プリミティブ」に寄る。

### 推奨

- **まずは案 A** で検討し、`world` / `game_logic` / `nif` に分割する。既存の `core` はそのまま利用し、`world` が「ランタイムの SoA と GameWorld」、`core` が「定数・敵種別・武器パラメータ・物理ユーティリティ」と役割を分ける。

---

## 4. 実施ステップ（検討項目）

本フェーズは「Step 番号」ではなく、**Rust lib 分割のための検討・実施項目**として以下を想定する。

| 項目 | 内容 |
|------|------|
| **R-LIB-1** | 現行 lib.rs のブロック切り出し順序の決定（型 → ヘルパー/AI → physics_step → NIF） |
| **R-LIB-2** | `world/` の作成: PlayerState, EnemyWorld, BulletWorld, ParticleWorld, ItemWorld（SoA）, BossState, GameWorldInner, GameWorld を移動 |
| **R-LIB-3** | `game_logic/` の作成: FrameEvent, physics_step_inner, drain_frame_events_inner, chase_ai, find_nearest_* を移動 |
| **R-LIB-4** | `nif/` の作成: 各 NIF を world_nif / game_loop_nif / save_nif 等に振り分け、lib.rs から呼び出す |
| **R-LIB-5** | lib.rs のスリム化: `mod` と `pub use`、`rustler::atoms!`、`rustler::init!` のみ残し、テスト・ビルドで動作確認 |
| **R-LIB-6** | ドキュメント更新: 本ドキュメントに「採用した構成」を記録し、STEPS_ALL の実施済みとしてマーク |

実施順序は **R-LIB-1 → R-LIB-2 → R-LIB-3 → R-LIB-4 → R-LIB-5 → R-LIB-6** を推奨。  
2D 固めや EOS 実装は、R-LIB 完了後に行う。

---

## 5. 関連ドキュメント

| ドキュメント | 用途 |
|-------------|------|
| [STEPS_ALL.md](./STEPS_ALL.md) | 全体ロードマップ・据え置き（3D/Slot）・Rust lib 分割の位置づけ |
| [STEPS_3D.md](./STEPS_3D.md) | Step 48〜54（据え置き） |
| [STEPS_SLOT_COMPONENT.md](./STEPS_SLOT_COMPONENT.md) | Step 55〜61（据え置き） |
| [EPIC_ONLINE_SERVICES.md](../06_system_design/EPIC_ONLINE_SERVICES.md) | Rust lib 整理・2D 固めの後に実装する EOS 設計 |
