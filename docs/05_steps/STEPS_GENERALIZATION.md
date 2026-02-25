# 1.4 汎用化（全9項）

**所属**: [STEPS_ALL.md](./STEPS_ALL.md) 1章 ゲームエンジン基礎 の 1.4 節。

**根拠**: [ENGINE_ANALYSIS_REVISED.md](../02_spec_design/ENGINE_ANALYSIS_REVISED.md)、[ELIXIR_RUST_DIVISION.md](../03_tech_decisions/ELIXIR_RUST_DIVISION.md)  
**方針**: 現状のヴァンサバ実装を活かしつつ、**他のゲームでも使える汎用エンジン**へ段階的に移行する

---

## 1.4 節 全体ロードマップ（1.4.1〜1.4.9）

| 項 | 目標 |
|----|------|
| 1.4.1 | Game インターフェース設計 |
| 1.4.2 | render_type 汎用化 |
| 1.4.3 | ゲーム切替（config） |
| 1.4.4 | ゲーム分離（vampire_survivor） |
| 1.4.5 | Game behaviour 実装 |
| 1.4.6 | エンジン API 安定化 |
| 1.4.7 | entity_registry（データ駆動） |
| 1.4.8 | ゲーム別アセットパス |
| 1.4.9 | 2 つ目のゲーム（ミニマル実装） |

---

## 目次

1. [現状の課題（汎用化の観点）](#1-現状の課題汎用化の観点)
2. [汎用化の目標](#2-汎用化の目標)
3. [1.4 汎用ゲームエンジン化（1.4.1〜1.4.9）](#3-step-3240-汎用ゲームエンジン化)
4. [推奨実施順序](#4-推奨実施順序)
5. [関連ドキュメント](#5-関連ドキュメント)

---

## 1. 現状の課題（汎用化の観点）

現状、エンジンとヴァンパイアサバイバーのゲームロジックが密結合している。

| レイヤー | ヴァンサバ固有の部分 | エンジンとして汎用化したい部分 |
|----------|----------------------|-------------------------------|
| **Rust** | `EnemyKind` (Slime, Bat, Golem)、`WeaponKind` (MagicWand, Axe...)、`BossKind`、`physics_step` 内の武器処理 | SoA、空間ハッシュ、フリーリスト、レンダラ、NIF ブリッジ |
| **Elixir** | `SpawnSystem` (ウェーブ間隔・敵種別)、`BossSystem` (ボススケジュール)、`LevelSystem` (経験値・武器選択)、`render_type` (:playing \| :level_up \| :boss_alert \| :game_over) | GameEvents、SceneManager、EventBus、FrameCache、InputHandler |

### 新ゲーム追加時の障壁

- **Rust**: `EnemyKind` / `WeaponKind` に enum を追加するたびに `lib.rs` を変更。新ゲームごとに分岐が増える
- **Elixir**: `SceneBehaviour.render_type/0` が `:playing | :level_up | :boss_alert | :game_over` に固定。タイトル画面や他ジャンルのシーン構造に対応しにくい

---

## 2. 汎用化の目標

1. **ゲームをプラグインとして差し替え可能にする**
   - 起動時に「どのゲームを動かすか」を指定できる
   - ヴァンサバは「ゲームの 1 つ」として存在する

2. **エンジン層とゲーム層の境界を明確にする**
   - エンジン: ゲームループ、シーン管理、描画、物理コア、NIF
   - ゲーム: 敵・武器の定義、スポーンロジック、シーン構成、UI

3. **新ゲーム追加時の変更を局所化する**
   - できるだけ Rust の enum を増やさない
   - ゲーム固有コードは `lib/games/<game_name>/` に集約

---

## 3. 1.4 汎用ゲームエンジン化（1.4.1〜1.4.9）

### フェーズ1: 境界の明確化（1.4.1〜1.4.3）

#### 1.4.1 Game インターフェース設計

**目標**: ゲームがエンジンに提供すべきインターフェースを文書化し、現状のヴァンサバがどうマッピングするか整理する。

**成果物**:
- `docs/06_system_design/GAME_INTERFACE.md` の作成
- Game behaviour の仮定義（Elixir）
- 現状の GameEvents / SceneManager / NIF が期待する「ゲーム側の責務」の一覧

**具体例（Game behaviour 案）**:
```elixir
@callback render_type() :: atom()           # シーン種別（ゲームが定義）
@callback initial_scenes() :: [scene_spec()] # 起動時のシーンスタック
@callback entity_registry() :: map()        # 敵・武器の ID → パラメータ（将来のデータ駆動用）
```

---

#### 1.4.2 render_type 汎用化

**目標**: `SceneBehaviour.render_type/0` の戻り値をゲームが定義できるようにする。

**現状**: `:playing | :level_up | :boss_alert | :game_over` にハードコード

**対応**:
- Rust 側 `GamePhase` を `u8` や `atom` に拡張し、ゲームが任意の `render_type` を返せるようにする
- レンダラが `render_type` に応じて描画モードを切り替える（現状はほぼ同じ。将来的にタイトル画面等で分岐）

**影響範囲**: `SceneBehaviour`、`FrameCache`、`lib.rs` の `get_frame_metadata`、レンダラ

---

#### 1.4.3 ゲーム切替（config）

**目標**: Application 起動時に「どのゲームを動かすか」を指定できるようにする。

**対応**:
- `config/config.exs` に `config :game, current: Game.VampireSurvivor` などを追加
- `Application` または `SceneManager` が `Application.get_env(:game, :current)` でゲームモジュールを取得
- 現状はヴァンサバ固定のまま、設定で切り替え可能にするだけ

**成果**: 将来的に `config :game, current: Game.RhythmGame` のように差し替え可能な土台

---

### フェーズ2: ゲームの分離（1.4.4〜1.4.6）

#### 1.4.4 ゲーム分離（vampire_survivor）✅ 実装済み

**目標**: ヴァンサバ固有のコードを `lib/games/vampire_survivor/` に集約する。

**対応**:
- `lib/game/` を `lib/engine/` と `lib/games/vampire_survivor/` に分割
- エンジン: `GameEvents`, `SceneManager`, `EventBus`, `FrameCache`, `InputHandler`, `StressMonitor`, `Stats`, `Telemetry`
- ヴァンサバ: `SpawnSystem`, `BossSystem`, `LevelSystem`, `Scenes.Playing`, `Scenes.LevelUp`, `Scenes.BossAlert`, `Scenes.GameOver`
- `Application` は `engine` と `games` の両方を監督

**注意**: 大規模リファクタのため、段階的に移動。まずはディレクトリ構造とモジュール配置の設計から。

---

#### 1.4.5 Game behaviour 実装 ✅ 実装済み

**目標**: 1.4.1 で設計した Game behaviour をヴァンサバが実装する。

**対応**:
- `Game.VampireSurvivor` が `@behaviour Game.Engine.Game` を実装
- `render_type/0`, `initial_scenes/0` などを実装
- `SceneManager` が `Game.Engine.Game` を参照して初期シーンを構築

---

#### 1.4.6 エンジン API 安定化 ✅ 実装済み

**目標**: ゲームがエンジンに依存する箇所をインターフェースとして明文化する。

**対応**:
- `Engine` モジュールで公開 API（`create_world`, `physics_step`, `push_scene`, `spawn_enemies` 等）を定義・ドキュメント化
- ゲームは `Engine` 経由でのみエンジンとやり取りする方針を文書化（`docs/06_system_design/ENGINE_API.md`）
- ヴァンサバを `Engine.*` 経由に移行済み

---

### フェーズ3: 拡張性の強化（1.4.7〜1.4.9、オプション）

#### 1.4.7 entity_registry（データ駆動）✅ 実装済み

**目標**: Rust の enum を増やさずに、ゲームが敵・武器を追加できるようにする（データ駆動の第一歩）。

**対応**:
- `EnemyKind` / `WeaponKind` / `BossKind` を `u8` ID にし、パラメータを `core/entity_params.rs` のテーブルで管理
- Elixir 側で `entity_registry/0` を実装し、atom → ID のマッピングを提供。Engine が ID に解決して NIF に渡す
- ヴァンサバは既存の Slime/Bat/Golem を ID 0,1,2、武器を 0〜5、ボスを 0,1,2 でマッピング

---

#### 1.4.8 ゲーム別アセットパス ✅ 実装済み

**目標**: ゲームごとにアセットのベースパスを切り替えられるようにする。

**対応**:
- `AssetLoader` に `with_game_assets(game_id)` API を追加
- `GAME_ASSETS_ID` 環境変数でゲーム別ディレクトリを指定（例: `assets/vampire_survivor/`）
- `Engine.Game` に `assets_path/0` コールバックを追加、Application 起動時に env を設定
- `bin/start.bat` で未設定時にデフォルト `vampire_survivor` を設定

---

#### 1.4.9 2 つ目のゲーム（ミニマル実装）

**目標**: 汎用化が機能していることを検証するために、極小の 2 つ目のゲームを実装する。

**例**: 「タイトル → プレイ（敵が直進するだけ）→ ゲームオーバー」のみのミニゲーム
- 新規 `lib/games/mini_shooter/` を作成
- `Game.MiniShooter` が `Game.Engine.Game` を実装
- 設定で `config :game, current: Game.MiniShooter` に切り替えて起動できることを確認

---

## 4. 推奨実施順序

```
【フェーズ1: 境界の明確化】
  1.4.1  Game インターフェース設計      ← 設計先行、コード変更は最小限
  1.4.2  render_type 汎用化
  1.4.3  ゲーム切替（config）

【フェーズ2: ゲームの分離】
  1.4.4  ゲーム分離（vampire_survivor）
  1.4.5  Game behaviour 実装
  1.4.6  エンジン API 安定化

【フェーズ3: 拡張性の強化（必要に応じて）】
  1.4.7  entity_registry（データ駆動）
  1.4.8  ゲーム別アセットパス
  1.4.9  2 つ目のゲーム（ミニマル実装）
```

**注意**: 1.4.4 はリファクタ規模が大きいため、1.4.1〜1.4.3 で設計を固めてから着手することを推奨する。

---

## 5. 関連ドキュメント

| ドキュメント | 用途 |
|-------------|------|
| [STEPS_ALL.md](./STEPS_ALL.md) | 全体ロードマップ・章・節・項構成 |
| [PRIORITY_STEPS.md](../04_roadmap/PRIORITY_STEPS.md) | 既存の優先度ロードマップ（P1〜P7, G1〜G3, Q1〜Q2） |
| [ENGINE_ANALYSIS_REVISED.md](../02_spec_design/ENGINE_ANALYSIS_REVISED.md) | エンジン現状の評価 |
| [ELIXIR_RUST_DIVISION.md](../03_tech_decisions/ELIXIR_RUST_DIVISION.md) | Elixir/Rust 役割分担、スコープ外・サポートしない項目 |
| [ASSET_MANAGEMENT.md](../06_system_design/ASSET_MANAGEMENT.md) | アセット管理設計（1.4.8 で拡張） |
