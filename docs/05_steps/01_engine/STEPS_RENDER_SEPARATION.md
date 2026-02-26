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

## 公開 API 案（初期 / game_window・game_render）

### `game_window` 側（winit 管理）

```rust
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
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

pub enum RenderError {
    EventLoopCreate(String),
    WindowCreate(String),
    SurfaceCreate(String),
    AdapterRequest(String),
    DeviceRequest(String),
    SurfaceAcquire(String),
    SurfaceLost,
    SurfaceOutdated,
    RenderBackend(String),
}

pub trait RenderBridge: Send + 'static {
    fn next_frame(&self) -> RenderFrame;
    fn on_move_input(&self, dx: f32, dy: f32);
    fn on_ui_action(&self, action: UiAction);
}

pub fn run_render_loop<B: RenderBridge>(
    bridge: B,
    config: WindowConfig,
) -> Result<(), RenderError>;
```

### `game_render` 側（wgpu 描画）

```rust
pub struct RendererInit {
    pub atlas_png: Vec<u8>,
}

pub struct Renderer;

impl Renderer {
    pub fn new(init: RendererInit, surface: &wgpu::Surface<'_>) -> Result<Self, RenderError>;
    pub fn resize(&mut self, width: u32, height: u32);
    pub fn render(&mut self, frame: &RenderFrame) -> Result<Option<UiAction>, RenderError>;
}
```

### 補足

- `run_render_loop` は `game_window` に所属し、EventLoop と入力イベントを管理する
- 解像度の**決定責務**は `game_window` が持つ
- `game_render` は `resize(w, h)` の**適用責務**のみ持つ
- 外部エラー型（`winit` / `wgpu`）は境界で `RenderError` に変換して返す
- 文字列ベースの UI アクションは段階的に `UiAction` enum へ移行する

---

## データフロー（分離後）

```
Elixir (GameEvents) ──NIF──► game_native (RenderBridge 実装)
                                  ▲
                                  │ next_frame / on_move_input / on_ui_action
                                  │
                          game_window (winit EventLoop)
                                  │
                                  ▼ render(frame), resize(w, h)
                            game_render (wgpu)
```

- `GameWorld` からのスナップショット生成は `game_native` 側で実施
- `game_window` は `RenderBridge` 経由で `game_native` を呼び出す（`next_frame` / `on_move_input` / `on_ui_action`）
- `game_render` は `RenderFrame` を受けて描画に専念
- `game_window` はプラットフォーム固有のイベントを吸収し、描画ループを進行する

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

---

## 1.8.6 Windows 動作確認と構成ドキュメント更新（実施）

**実施日**: 2026-02-26

### 動作確認結果

- Windows で `iex -S mix` から起動し、ウィンドウ表示・描画更新・入力反映・UI 操作が成立することを確認。
- `game_window`（winit 管理）と `game_render`（wgpu 描画）が分離された状態で、`game_native` は NIF 境界と `GameWorld` 連携に集中できていることを確認。
- 1.8 の到達目標（責務分離後の実動作確認）は達成とする。

### 確認観点（1.8.6）

| 観点 | 結果 |
|------|------|
| `iex -S mix` 単体起動でウィンドウ表示 | OK |
| WASD/矢印キー入力の反映 | OK |
| HUD 操作（Start/Retry/Save/Load/武器選択） | OK |
| `game_native` から描画実装依存の分離 | OK |

### 更新ドキュメント

- `docs/06_system_design/FOLDER_CONNECTIONS.md` を 1.8 完了後の構成（`game_native` / `game_window` / `game_render` の責務分離）に更新。
- `docs/06_system_design/ARCHITECTURE.md` を分離後のエンジン内部構成に合わせて更新。
- 本書（`STEPS_RENDER_SEPARATION.md`）に 1.8.6 の実施記録を追記。

