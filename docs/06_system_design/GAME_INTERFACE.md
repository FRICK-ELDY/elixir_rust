# Game インターフェース設計

**対象**: Step 32（汎用ゲームエンジン化）  
**根拠**: [STEPS_GENERALIZATION.md](../05_steps/01_engine/STEPS_GENERALIZATION.md)、[ENGINE_ANALYSIS_REVISED.md](../02_spec_design/ENGINE_ANALYSIS_REVISED.md)、[ELIXIR_RUST_DIVISION.md](../03_tech_decisions/ELIXIR_RUST_DIVISION.md)

> **エンジン名前空間**: 本文では **Engine** を使用する。`Engine.Game` で「Game」の重複を避けつつ、エンジンとゲームの境界を明確にする。

---

## 1. 目標

ゲームがエンジンに提供すべきインターフェースを文書化し、エンジン層とゲーム層の境界を明確にする。

- エンジン: ゲームループ、シーン管理、描画、物理コア、NIF
- ゲーム: 敵・武器の定義、スポーンロジック、シーン構成、UI

---

## 2. Game behaviour の仮定義

### 2.1 モジュール配置（案）

```
# 将来の構成例
lib/
├── engine/                # エンジン名前空間
│   ├── game.ex            # Engine.Game behaviour の定義
│   ├── game_loop.ex
│   └── scene_manager.ex
└── games/
    └── vampire_survivor/
        └── game.ex        # Game.VampireSurvivor @impl Engine.Game
```

### 2.2 Game behaviour コールバック（案）

```elixir
defmodule Engine.Game do
  @moduledoc """
  ゲームがエンジンに提供すべきインターフェース。
  エンジンはこの behaviour を実装したモジュールを config で指定して起動する。
  """

  # シーン種別（ゲームが定義。Rust の GamePhase / 描画モードに渡す）
  @callback render_type() :: atom()

  # 起動時のシーンスタック（例: [Playing] または [Title, Playing]）
  @callback initial_scenes() :: [scene_spec()]

  # 敵・武器の ID → パラメータ（将来のデータ駆動用。現状は未使用）
  @callback entity_registry() :: map()

  # 物理演算を実行するシーンモジュールの一覧（maybe_run_physics の汎用化用）
  @callback physics_scenes() :: [module()]

  # ゲームメタデータ（ウィンドウタイトル等に利用）
  @callback title() :: String.t()
  @callback version() :: String.t()

  # コンテキストのデフォルト値（build_context にマージ。wave, difficulty 等）
  @callback context_defaults() :: map()

  # （オプション）Step 33〜39 で検討
  # @callback assets_base_path() :: String.t()   # Step 39: ゲーム別アセットパス
  # @callback tick_rate_ms() :: pos_integer()    # 60Hz 以外への対応
end
```

### 2.3 型定義（案）

```elixir
# scene_spec: 初期シーンを表現
# %{module: module(), init_arg: term()}
@type scene_spec :: %{module: module(), init_arg: term()}
```

### 2.4 設計上の検討事項

#### 物理演算の実行タイミング

| 現状 | 対応案 |
|------|--------|
| `maybe_run_physics` が `Game.Scenes.Playing` をハードコード | `physics_scenes/0` でゲームが物理実行シーンの一覧を返す |

#### 遷移時の特別処理

| 現状 | 対応案 |
|------|--------|
| `process_transition` が BossAlert, GameOver をハードコード（telemetry, spawn_boss 等） | A) シーンの update 内で完結させる B) `on_transition/3` コールバックでゲームに委譲 |

#### コンテキストの拡張

| 現状 | 対応案 |
|------|--------|
| `build_context` が固定のキーのみ返す | `context_defaults/0` でゲーム固有の値（wave, difficulty 等）を追加 |

---

## 3. エンジンがゲームに期待する責務

### 3.1 GameEvents が期待するもの

| 責務 | 現状の実装 | 備考 |
|------|------------|------|
| （TBD） | | 物理演算は Playing シーンでのみ実行 |
| （TBD） | | `maybe_run_physics` が `Game.Scenes.Playing` をハードコード |
| （TBD） | | 武器選択・リスタート時の NIF 呼び出し |
| （TBD） | | `process_transition` が `Game.Scenes.BossAlert`, `Game.Scenes.GameOver` をハードコード |

### 3.2 SceneManager が期待するもの

| 責務 | 現状の実装 | 備考 |
|------|------------|------|
| 初期シーンの決定 | `init/1` で `Game.Scenes.Playing` を固定 | `initial_scenes/0` でゲームが返す |
| シーンモジュールが SceneBehaviour を実装 | Playing, LevelUp, BossAlert, GameOver | init/1, update/2, render_type/0 |
| render_type の取得 | 各シーンの `render_type/0` | 任意の atom を返せる（Step 33 で汎用化済み） |

### 3.3 NIF（NifBridge）が期待するもの

| 責務 | 現状の実装 | 備考 |
|------|------------|------|
| world_ref の管理 | `GameEvents` が create_world で取得 | ゲーム開始時にエンジンが作成 |
| 敵種別・武器種別 | `EnemyKind`, `WeaponKind` が Rust にハードコード | 将来は entity_registry 経由で ID 参照 |
| render_type の利用 | `get_frame_metadata` → FrameCache → レンダラ | 描画モード切り替え用 |
| （その他 NIF 呼び出し） | spawn_enemies(kind), add_weapon(name), spawn_boss(kind) など | kind/name は現状 atom/string |

---

## 4. 現状のヴァンサバとのマッピング

### 4.1 Game behaviour との対応（現状）

| コールバック | ヴァンサバでの実装 | 備考 |
|--------------|-------------------|------|
| `render_type/0` | シーンごとに異なる（`:playing` 等） | ゲーム単位ではなくシーン単位で定義されている |
| `initial_scenes/0` | 未実装 | SceneManager が Playing を直接指定 |
| `entity_registry/0` | 未実装 | 将来のデータ駆動用 |
| `physics_scenes/0` | 未実装 | 現状は Playing のみ。汎用化で [Playing] を返す |
| `title/0`, `version/0` | 未実装 | ウィンドウタイトル等 |
| `context_defaults/0` | 未実装 | ゲーム固有の context 拡張 |

### 4.2 シーン構成（ヴァンサバ現状）

```
起動時: [Playing]
遷移:   Playing → LevelUp (push) → Playing (pop)
        Playing → BossAlert (push) → Playing (pop)
        Playing → GameOver (replace) → Playing (replace, リスタート時)
```

### 4.3 ゲーム固有モジュール（ヴァンサバ）

| モジュール | 役割 |
|------------|------|
| `Game.Scenes.Playing` | メインゲーム、スポーン・ボス・レベルアップの orchestration |
| `Game.Scenes.LevelUp` | 武器選択 UI |
| `Game.Scenes.BossAlert` | ボス登場演出 |
| `Game.Scenes.GameOver` | ゲームオーバー・リスタート |
| `Game.SpawnSystem` | 敵スポーンロジック |
| `Game.BossSystem` | ボス登場タイミング |
| `Game.LevelSystem` | 経験値・武器選択 UI |

---

## 5. 将来の拡張ポイント

### 5.1 Step 33〜38 で実装予定

- [x] `render_type` を atom の union から任意の atom に汎用化（Step 33）
- [x] `initial_scenes/0` でゲームが起動シーンを指定（Step 34, 36）
- [x] `physics_scenes/0` で物理演算対象シーンを抽象化（Step 35, 36）
- [ ] 遷移時の特別処理の責務分離（シーン内完結 or on_transition コールバック）
- [x] `entity_registry/0` で ID ベースの敵・武器参照（Step 38）
- [ ] ゲーム登録メカニズム `config :game, current: Game.VampireSurvivor`（Step 34）

### 5.2 将来フェーズで検討

| 項目 | 用途 |
|------|------|
| 入力マッピング | キーバインド（ジャンプ・攻撃等）の定義 |
| シーン別 BGM/SE | シーン開始時の音声指定 |
| セーブ/ロード | 永続化用シリアライズ API |
| HUD レイアウト | get_frame_metadata の拡張 or カスタム HUD 定義 |
| `assets_base_path/0` | Step 39: ゲーム別アセットパス |
| `tick_rate_ms/0` | 60Hz 以外の tick レート |

### 5.3 現時点スコープ外

- ローカライゼーション
- 解像度・フルスクリーン等の画面設定
- エラー時のフォールバック（タイトルへ戻る等）

---

## 6. 関連ドキュメント

| ドキュメント | 用途 |
|-------------|------|
| [STEPS_GENERALIZATION.md](../05_steps/01_engine/STEPS_GENERALIZATION.md) | Step 32〜40 のロードマップ |
| [ELIXIR_RUST_DIVISION.md](../03_tech_decisions/ELIXIR_RUST_DIVISION.md) | Elixir/Rust 役割分担 |
| [ASSET_MANAGEMENT.md](./ASSET_MANAGEMENT.md) | アセット管理（Step 39 でゲーム別パス） |
