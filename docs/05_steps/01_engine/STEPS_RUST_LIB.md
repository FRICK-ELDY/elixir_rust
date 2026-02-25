# 1.6 Rust lib 分割・整理（全9項）

**所属**: [STEPS_ALL.md](../STEPS_ALL.md) 1章 エンジン構築 の 1.6 節。

**目的**: 1.9（3D）・1.10（Slot・コンポーネント）に着手する**前**に、Rust 側を **game_core / game_native / game_window の 3 クレート構成**に再編し、`lib.rs` を分割・整理して保守性と拡張性を高める。  
**前提**: 1.1〜1.5 の拡張フェーズまで一通り完了していること。現状の 1 パッケージ（lib + bin 同居）をワークスペース分離し、共通ロジックを game_core に集約する。

**実施タイミング**: 1.9・1.10 は**据え置き**とし、本「Rust lib 分割・整理」→ **1.7 2D ゲームの固め** → [EPIC_ONLINE_SERVICES.md](../../06_system_design/EPIC_ONLINE_SERVICES.md) の実装、の順で進める。

---

## 1.6 節 全体ロードマップ（1.6.1〜1.6.9）

| 項 | 目標 |
|----|------|
| 1.6.1 | **Workspace Layout ツール**: xtask による WorkspaceLayout.md 自動生成とファイルヘッダー規約 |
| 1.6.2 | Workspace 化: game_core / game_native / game_window の 3 クレート構成に分割 |
| 1.6.3 | ブロック切り出し順序の決定 |
| 1.6.4 | `world/` の作成と型定義の移動 |
| 1.6.5 | `game_logic/` の作成とロジックの移動 |
| 1.6.6 | `nif/` の作成と NIF 関数の移動 |
| 1.6.7 | `lib.rs` のスリム化と動作確認 |
| 1.6.8 | Elixir・Mix のビルドパス確認 |
| 1.6.9 | ドキュメント更新 |

> 3D・Slot の**前**に実施。1.1〜1.5 完了後に着手する。

---

## 目次

1. [Workspace Layout ツール（1.6.1）](#1-workspace-layout-ツール161)
2. [現状の整理](#2-現状の整理)
3. [分割・整理の目標](#3-分割整理の目標)
4. [フォルダ構成案（採用: 3 クレート + 機能別）](#4-フォルダ構成案採用-3-クレート--機能別)
5. [実施ステップ（検討項目）](#5-実施ステップ検討項目)
6. [関連ドキュメント](#6-関連ドキュメント)

---

## 1. Workspace Layout ツール（1.6.1）

ファイル分散により AI の精度が落ちるのを防ぐため、**xtask** でコード全体の Path・Lines・Status・Summary を把握する Workspace Layout ツールを導入する。

### 1.1 コーディングルール（ファイルヘッダー）

各ソースファイルの**先頭**に以下を記述する。

```
//! Path: <リポジトリルートからの相対パス>
//! Summary: <1行でファイルの責務・内容を要約>
```

- **Path**: 当該ファイルのリポジトリルートからの相対パス。例: `native/game_native/src/lib.rs`
- **Summary**: 日本語または英語で 1 行要約。AI が文脈を把握しやすくする。

#### 対象言語とコメント形式

| 言語 | 例 |
|------|-----|
| Rust | `//! Path: ...` `//! Summary: ...` |
| Elixir | `# Path: ...` `# Summary: ...` |
| その他 | 該当言語のブロック/行コメントで同様に記述 |

### 1.2 Status 基準（行数による分割優先度）

| 行数 | Status | 記号 | 意味 |
|------|--------|------|------|
| 0–4 | 0 | ⚪ | 無評価 |
| 5–50 | 1 | 🟢 | OK、保持 |
| 51–100 | 2 | 🟡 | 様子見、早めに分割候補 |
| 101–200 | 3 | 🟠 | 分割推奨 |
| 201– | 4 | 🔴 | 最優先で分割 |

### 1.3 xtask ツール仕様

- **コマンド**: `cargo xtask workspace-layout`（プロジェクトルートまたは native/ から実行）
- **出力**: `WorkspaceLayout.md` をプロジェクトルートに自動生成
- **出力形式**:

```markdown
# Workspace Layout（自動生成）

| Path | Lines | Status | Summary |
|------|-------|--------|---------|
| native/game_native/src/lib.rs | 1680 | 🔴 | NIF エントリ・ワールド型・物理ステップ・イベント・セーブをすべて含む |
| native/game_native/src/main.rs | 1384 | 🔴 | スタンドアロン描画ループ・ウィンドウ（winit/wgpu） |
...
```

- **スキャン対象**: `native/`, `lib/` 以下の `.rs`, `.ex`, `.exs` および指定した拡張子
- **Summary 取得**: ファイルヘッダーの `Summary:` をパース。未記述の場合は `(未設定)` と出力
- **Lines**: 空行・コメントを除いた有効行数（コード行）

### 1.4 xtask の配置

1.6.2 で workspace 化する際、`xtask` を workspace メンバーとして追加する。

```
native/
├── Cargo.toml          # workspace（members に xtask を含む）
├── xtask/
│   ├── Cargo.toml      # [[bin]] workspace-layout
│   └── src/
│       └── main.rs
├── game_core/
├── game_native/
└── game_window/
```

1.6.1 時点で workspace が未整備の場合は、`xtask/` を単体クレートとして `native/xtask/` に作成し、`cargo run -p xtask -- workspace-layout` で実行可能にする。

### 1.5 実施内容（1.6.1）

1. xtask クレートを作成し、`workspace-layout` サブコマンドを実装
2. 本コーディングルールを `.cursor/rules` またはプロジェクト RULE に追記
3. 既存ファイルに Path・Summary ヘッダーを段階的に追加（少なくとも lib.rs, main.rs から開始）
4. `cargo xtask workspace-layout` 実行で `WorkspaceLayout.md` が生成されることを確認

---

## 2. 現状の整理

### 2.1 現在の `game_native/src` 構成（1.6.2 で 3 クレートに分割予定）

```
native/game_native/src/
├── lib.rs          # 約 1400 行。NIF・ワールド型・物理ステップ・イベント・セーブ等をすべて含む
├── main.rs         # スタンドアロン描画ループ・ウィンドウ → game_window へ移動
├── audio.rs        # BGM・SE（rodio）→ game_window へ移動
├── core/           # ゲームロジック共通 → game_core へ移動
│   ├── mod.rs
│   ├── boss.rs, enemy.rs, entity_params.rs, constants.rs, util.rs, weapon.rs, item.rs
│   └── physics/
│       ├── mod.rs, spatial_hash.rs, separation.rs, obstacle_resolve.rs, rng.rs
├── renderer/       # → game_window へ移動
│   ├── mod.rs
│   └── shaders/
├── asset/          # → game_window へ移動
│   └── mod.rs
```

### 2.2 lib.rs に集約されている主なブロック

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

## 3. 分割・整理の目標

- **3 クレート構成**: `game_core`（共通ロジック）・`game_native`（NIF ライブラリ）・`game_window`（スタンドアロンバイナリ）に分離し、責務と依存を明確にする。
- **責務の分離**: game_native 内で「ワールド状態の型（world）」「物理ステップなどのゲームロジック（game_logic）」「NIF のエントリポイント（nif）」を別モジュールに分け、lib.rs は `mod` と `pub use` および `rustler::init!` のみに近づける。
- **2D 固め・3D 拡張の土台**: 共通型を game_core に集約し、のちに 3D 用モジュールや Slot スナップショット用の型を追加しやすい構成にする。
- **後方互換**: Elixir 側の `App.NifBridge` の NIF 名・引数・戻り値は変えず、Rust 内部のクレート分割・ファイル分割のみ行う。

---

## 4. フォルダ構成案（採用: 3 クレート + 機能別）

### 4.1 ワークスペース構成（game_core / game_native / game_window）

```
native/
├── Cargo.toml                    # workspace root
├── game_core/                    # 共通ロジック（lib）
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── constants.rs, entity_params.rs, util.rs
│       ├── enemy.rs, boss.rs, item.rs, weapon.rs
│       └── physics/
│           ├── mod.rs
│           ├── spatial_hash.rs, separation.rs, obstacle_resolve.rs, rng.rs
│           └── ...
├── game_native/                  # NIF ライブラリ（Elixir 連携、game_core に依存）
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # mod 宣言、pub use、rustler::atoms!、rustler::init!
│       ├── world/                # ワールド型と SoA 構造体
│       │   ├── mod.rs
│       │   ├── player.rs         # PlayerState
│       │   ├── enemy.rs          # EnemyWorld (+ EnemySeparation impl)
│       │   ├── bullet.rs, particle.rs
│       │   ├── boss.rs           # BossState（core の BossKind/Params と区別）
│       │   ├── item_world.rs
│       │   └── game_world.rs     # GameWorldInner, GameWorld
│       ├── game_logic/           # 物理ステップとその補助
│       │   ├── mod.rs
│       │   ├── physics_step.rs
│       │   ├── chase_ai.rs
│       │   └── events.rs
│       └── nif/                  # NIF エントリのみ
│           ├── mod.rs
│           ├── world_nif.rs
│           ├── game_loop_nif.rs
│           └── save_nif.rs
└── game_window/                  # スタンドアロンバイナリ（game_core に依存、Elixir 非依存）
    ├── Cargo.toml
    └── src/
        ├── main.rs
        ├── audio.rs
        ├── renderer/
        ├── asset/
        └── ...                   # 独自の GameWorld 実装（main 側）
```

### 4.2 依存関係

| クレート | 依存 | 役割 |
|----------|------|------|
| game_core | （なし） | 定数・敵種別・武器パラメータ・物理プリミティブ |
| game_native | game_core, rustler, rayon | NIF、ワールド型、物理ステップ、Elixir 連携 |
| game_window | game_core, winit, wgpu, egui, rodio | スタンドアロン描画・音声・入力 |

### 4.3 利点・注意

- **利点**: 共通ロジックを game_core に集約し、重複排除が徹底する。lib と bin の責務・依存が明確になり、3D・Slot 追加時に game_core に型を足しやすい。
- **注意**: `renderer/`, `asset/`, `audio` は game_window 専用。Elixir の Mix は `game_native` を NIF 用クレートとして参照する設定を維持する。

---

## 5. 実施ステップ（検討項目）

本フェーズは **Workspace Layout ツールの整備** → **3 クレート構成への移行** → **game_native 内のモジュール分割** を順に実施する。

| 項 | 内容 |
|----|------|
| 1.6.1 | **Workspace Layout ツール**: xtask 作成、Path/Summary ヘッダー規約の策定、WorkspaceLayout.md 自動生成を実装。既存主要ファイル（lib.rs, main.rs 等）にヘッダーを追加 |
| 1.6.2 | **Workspace 化**: native/Cargo.toml を workspace 化。game_core を新規作成し、core/ と physics/ を移動。game_native を lib 専用にし、game_window を新規クレートとして main.rs・renderer・asset・audio を移動 |
| 1.6.3 | 現行 lib.rs のブロック切り出し順序の決定（型 → ヘルパー/AI → physics_step → NIF） |
| 1.6.4 | `world/` の作成: PlayerState, EnemyWorld, BulletWorld, ParticleWorld, ItemWorld（SoA）, BossState, GameWorldInner, GameWorld を移動 |
| 1.6.5 | `game_logic/` の作成: FrameEvent, physics_step_inner, drain_frame_events_inner, chase_ai, find_nearest_* を移動 |
| 1.6.6 | `nif/` の作成: 各 NIF を world_nif / game_loop_nif / save_nif 等に振り分け、lib.rs から呼び出す |
| 1.6.7 | lib.rs のスリム化: `mod` と `pub use`、`rustler::atoms!`、`rustler::init!` のみ残し、テスト・ビルドで動作確認 |
| 1.6.8 | Elixir・Mix のビルドパス確認: rustler が game_native を正しく参照するよう設定を更新 |
| 1.6.9 | ドキュメント更新: 本ドキュメントに「採用した構成」を記録し、STEPS_ALL の実施済みとしてマーク |

実施順序は **1.6.1 → 1.6.2 → 1.6.3 → 1.6.4 → 1.6.5 → 1.6.6 → 1.6.7 → 1.6.8 → 1.6.9** を推奨。  
1.7 2D 固めや 1.8 EOS 実装は、1.6 完了後に行う。

---

## 6. 関連ドキュメント

| ドキュメント | 用途 |
|-------------|------|
| [STEPS_ALL.md](../STEPS_ALL.md) | 全体ロードマップ・据え置き（3D/Slot）・Rust lib 分割の位置づけ |
| [STEPS_3D.md](./STEPS_3D.md) | 1.9（据え置き） |
| [STEPS_SLOT_COMPONENT.md](./STEPS_SLOT_COMPONENT.md) | 1.10（据え置き） |
| [EPIC_ONLINE_SERVICES.md](../../06_system_design/EPIC_ONLINE_SERVICES.md) | Rust lib 整理・2D 固めの後に実装する EOS 設計 |
