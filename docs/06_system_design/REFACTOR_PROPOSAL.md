# リファクタリング提案書

**対象**: `native/game_native/src/` のクレート構造  
**実施タイミング**: Step 15（ゲームオーバー・リスタート）完了後  
**ステータス**: 提案中

---

## 背景・課題

現在の `src/` は以下の構成になっている。

```
native/game_native/src/
├── lib.rs        → [lib] cdylib … Rustler NIF エントリポイント
├── main.rs       → [[bin]]      … wgpu 描画ウィンドウ起動
└── renderer/
    ├── mod.rs    → GPU 描画コード（main.rs が mod renderer; で取り込む）
    └── shaders/
        └── sprite.wgsl
```

`lib.rs`（NIF）と `main.rs`（描画）が同じ `src/` に共存しており、  
Step 7〜15 でゲームロジック・物理演算・AI などのコードが増えるにつれ、  
「どちらのエントリポイントに属するコードか」が曖昧になるリスクがある。

具体的な懸念点：

| 懸念 | 詳細 |
|---|---|
| 責務の混在 | `renderer/` は現在 `main.rs` 管轄だが、Step 7 以降は Elixir → NIF → Renderer の流れになる |
| モジュール参照の不整合 | `lib.rs` から `renderer` を使いたい場合、`mod renderer;` を重複宣言できない |
| 可読性の低下 | 新規参加者が `lib.rs` と `main.rs` の役割分担を把握しにくい |

---

## 提案：`renderer` を `lib.rs` 側に移動する

### 変更後の構造

```
native/game_native/src/
├── lib.rs            → [lib] cdylib … NIF エントリポイント + pub mod renderer
├── main.rs           → [[bin]]      … ウィンドウ起動のみ（lib の型を use）
├── renderer/
│   ├── mod.rs        → GPU 描画コード（lib.rs 経由で公開）
│   └── shaders/
│       └── sprite.wgsl
└── physics/          ← Step 9〜12 で追加予定
    ├── movement.rs
    └── spatial_hash.rs
```

### 変更内容

#### `lib.rs`（変更後）

```rust
pub mod renderer;   // ← 追加：renderer を lib 側で管理

use rustler::NifResult;

#[rustler::nif]
fn add(a: i64, b: i64) -> NifResult<i64> {
    Ok(a + b)
}

rustler::init!("Elixir.Game.NifBridge");
```

#### `main.rs`（変更後）

```rust
// mod renderer; を削除し、lib クレートの pub mod を use する
use game_native::renderer::Renderer;

use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

// ... 以降は現行と同じ ...
```

#### `Cargo.toml`（変更なし）

```toml
[lib]
name = "game_native"
crate-type = ["cdylib"]

[[bin]]
name = "game_window"
path = "src/main.rs"
```

`Cargo.toml` の変更は不要。`[[bin]]` は `[lib]` と同じクレートの成果物を参照できる。

---

## 変更しない理由（現状維持の選択肢）

| 観点 | 評価 |
|---|---|
| Cargo の仕様 | `lib.rs` + `main.rs` 共存は Rust 公式が想定する正式な構成 |
| 変更コスト | Step 7〜15 の実装中にリファクタリングを挟むと差分が複雑になる |
| 動作への影響 | 現状でも `cargo run --bin game_window` と `mix compile` は独立して動作する |

**Step 15 完了まで現状維持を推奨する。**

---

## 別案：Workspace で 2 クレートに分割

```
native/
├── Cargo.toml          ← workspace 定義
├── game_native/        ← [lib] cdylib … NIF + ゲームロジック
│   └── src/lib.rs
└── game_window/        ← [[bin]]      … ウィンドウ・描画
    └── src/main.rs
```

### メリット

- 責務が完全に分離される
- `game_window` が `game_native` に依存する形が明示される
- 将来的に Linux / macOS 対応クレートを追加しやすい

### デメリット

- `Cargo.toml` を Workspace 構成に書き換える必要がある
- Rustler の `crate:` 指定も合わせて変更が必要
- 現段階では**過剰な分割**になる可能性が高い

→ Step 15 完了後に改めて検討する。

---

## 実施計画（Step 15 完了後）

```
リファクタリング用ブランチ: refactor/crate-structure
```

| # | 作業 | 影響ファイル |
|---|---|---|
| 1 | `lib.rs` に `pub mod renderer;` を追加 | `src/lib.rs` |
| 2 | `main.rs` の `mod renderer;` を削除し `use game_native::renderer::Renderer;` に変更 | `src/main.rs` |
| 3 | `cargo build` でコンパイルエラーがないことを確認 | — |
| 4 | `mix compile` で NIF が正常にビルドされることを確認 | — |
| 5 | `cargo run --bin game_window` で描画が正常に動作することを確認 | — |
| 6 | `STEPS.md` の Step 2〜5 のコードサンプルを更新 | `docs/05_steps/STEPS.md` |

**所要時間の見積もり**: 1〜2 時間（テスト含む）

---

## 判断基準

以下の条件をすべて満たした時点でリファクタリングを実施する。

- [ ] Step 15 が完了し、全機能が動作確認済みであること
- [ ] `feat/step15-game-over` ブランチが `main` にマージ済みであること
- [ ] リファクタリング後も全チェックポイントが通過すること
