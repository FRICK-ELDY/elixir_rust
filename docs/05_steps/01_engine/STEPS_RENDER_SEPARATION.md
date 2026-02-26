# 1.8 描画責務分離（game_native → game_window / game_render）

**所属**: [STEPS_ALL.md](../STEPS_ALL.md) 1章 エンジン構築 の 1.8 節。

**目的**: 1.7 で `game_native` に統合した描画処理を再整理し、責務を以下に分離する。

- `game_window`: `winit` 管理、サイズ決定、イベント処理
- `game_render`: `wgpu` パイプライン、描画コマンド、`render(frame)`、`resize(w, h)`
- `game_native`: NIF 境界、`GameWorld`、ゲームループ、Bridge

---

## 概要

| 項目 | 内容 |
|------|------|
| **狙い** | 依存方向の明確化、保守性・テスト容易性の向上、将来の差し替え容易化 |
| **依存関係** | `game_window -> game_render`、`game_native -> (game_window, game_render)` |
| **重要な考え方** | 「ウィンドウ表示」と「描画」は分離可能。ただしオンスクリーン描画では `Surface` がウィンドウに依存するため、接点は維持する |
| **対象プラットフォーム** | まずは Windows。macOS/Linux は後続で検証 |

---

## 責務分離（確定方針）

### `game_window` の責務

- `winit::EventLoop` のライフサイクル管理
- ウィンドウ作成・タイトル・初期サイズの決定
- `KeyboardInput` / `Resized` / `Focused` などのイベント処理
- 必要に応じて `request_redraw` を発行

### `game_render` の責務

- `wgpu::Device/Queue/Pipeline` の初期化
- インスタンス更新（sprites / particles / items / obstacles）
- `render(frame)` の実行
- `resize(width, height)` の適用
- HUD 描画（egui）

### `game_native` の責務

- NIF エントリ (`start_render_thread` など)
- `ResourceArc<GameWorld>` の保持
- `world.read()` から `RenderFrame` スナップショットを構築
- 入力・UI アクションをゲームループと接続

---

## 1.8 節 全体ロードマップ（1.8.1〜1.8.6）

| 項 | 目標 |
|----|------|
| **1.8.1** | 分離方針と依存方向の確定（責務・公開 API・データ受け渡し） |
| **1.8.2** | `native/game_render` 作成（描画コアを移設） |
| **1.8.3** | `native/game_window` 作成（ウィンドウ・イベント層を移設） |
| **1.8.4** | Bridge 実装（frame/input/ui_action の接続） |
| **1.8.5** | `game_native` から描画実装依存を削減（NIF/状態管理に集中） |
| **1.8.6** | Windows 動作確認と構成ドキュメント更新 |

---

## 公開 API 案（初期）

### `game_render` 側

```rust
pub struct RenderConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub atlas_png: Vec<u8>,
}

pub struct RenderFrame {
    pub sprites: Vec<SpriteInstanceIn>,
    pub particles: Vec<ParticleIn>,
    pub items: Vec<ItemIn>,
    pub obstacles: Vec<ObstacleIn>,
    pub camera_offset: (f32, f32),
    pub hud: HudDataIn,
}

pub enum UiAction {
    Start,
    Retry,
    Save,
    Load,
    LoadConfirm,
    LoadCancel,
    ChooseWeapon(String),
}

pub trait RenderBridge: Send + 'static {
    fn next_frame(&self) -> RenderFrame;
    fn on_move_input(&self, dx: f32, dy: f32);
    fn on_ui_action(&self, action: UiAction);
}

pub fn run_render_loop<B: RenderBridge>(
    bridge: B,
    config: RenderConfig,
) -> Result<(), RenderError>;
```

### 補足

- 解像度の**決定責務**は `game_window` が持つ
- `game_render` は `resize(w, h)` の**適用責務**のみ持つ
- 文字列ベースの UI アクションは段階的に `UiAction` enum へ移行する

---

## データフロー（分離後）

```
Elixir (GameEvents) ──NIF──► game_native
                              │
                              ├─ world.read() -> RenderFrame
                              ├─ on_move_input(dx, dy)
                              └─ on_ui_action(action)
                                      ▲
                                      │
                       game_window (winit) -> game_render (wgpu)
```

- `GameWorld` からのスナップショット生成は `game_native` 側で実施
- `game_render` は `RenderFrame` を受けて描画に専念
- `game_window` はプラットフォーム固有のイベントを吸収

---

## 実施時の注意点

- `game_render` に `rustler` や `ResourceArc` を持ち込まない
- 毎フレームの受け渡しで不要なコピーを増やさない（事前容量確保・再利用を検討）
- `request_redraw` の戦略を明示し、ビジーループを避ける
- `UiAction` の互換レイヤーを一時的に置き、既存 Elixir 側イベント契約を壊さない

---

## 完了条件（1.8）

- `game_window` と `game_render` の責務境界がコード上で明確
- `game_native` は NIF 境界と状態管理に集中し、描画詳細依存を最小化
- Windows で `iex -S mix` 起動から入力・描画・UI 操作が成立
- `FOLDER_CONNECTIONS.md` / `ARCHITECTURE.md` が新構成を反映

