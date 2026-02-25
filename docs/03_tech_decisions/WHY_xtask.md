# なぜ xtask をビルド・ツール実行に採用するのか

このドキュメントでは、Rust ワークスペースにおけるビルド補助・ツール実行の仕組みとして xtask を採用した技術的根拠と、本プロジェクトでの利用内容を説明します。

---

## 目次

1. [xtask とは](#1-xtask-とは)
2. [採用理由](#2-採用理由)
3. [本プロジェクトでの構成](#3-本プロジェクトでの構成)
4. [Workspace Layout ツール](#4-workspace-layout-ツール)
5. [実行方法](#5-実行方法)
6. [関連ドキュメント](#6-関連ドキュメント)

---

## 1. xtask とは

xtask は、Rust プロジェクトにおける**カスタムタスク・ツール実行**のための慣習的な仕組みです。

- **配置**: ワークスペース内の 1 クレート（`xtask/`）として、メインのライブラリ・バイナリとは別に配置する
- **実行**: `cargo run -p xtask -- <サブコマンド>` で任意の Rust バイナリを実行
- **用途**: コード生成、ドキュメント生成、リント補助、ビルドスクリプトなど
- **利点**: Makefile やシェルスクリプトに依存せず、Rust の型安全性とクロスプラットフォーム性を活かしてツールを実装できる

`cargo xtask` というサブコマンド形式で使う場合は、`cargo install cargo-xtask` で別途インストールする必要があります。本プロジェクトでは `cargo run -p xtask -- ...` で十分であり、追加インストールは不要です。

---

## 2. 採用理由

### 2.1 ビルドシステムへの組み込み

Cargo ワークスペースの一員として配置するため、`cargo build` と同様の環境でツールをビルド・実行できます。依存関係の管理も Cargo.toml で一元化できます。

### 2.2 クロスプラットフォーム対応

Rust で書かれた xtask は Windows / macOS / Linux で同じように動作します。シェルスクリプトの行末やパス区切りに悩む必要がありません。

### 2.3 既存エコシステムとの親和性

[Rust の xtask パターン](https://github.com/matklad/cargo-xtask) は多くのプロジェクトで採用されており、コミュニティで広く知られた慣習です。

### 2.4 他手段との比較

| 手段 | メリット | デメリット |
|------|----------|------------|
| **xtask** | Cargo に統合、型安全、クロスプラットフォーム | Rust のビルドが必要 |
| Makefile | 軽量、速い | Unix 前提、構文が古い |
| シェルスクリプト (.bat / .sh) | 単純な処理には向く | プラットフォーム別の用意が必要 |
| Python スクリプト | 柔軟 | Python 実行環境の依存 |

本プロジェクトでは、Workspace Layout の生成のように**ファイル走査・パース・Markdown 出力**といった処理を実装する必要があり、Rust による xtask が適しています。

---

## 3. 本プロジェクトでの構成

### 3.1 ワークスペース構成

```
native/
├── Cargo.toml          # workspace（game_native, xtask）
├── game_native/        # NIF ライブラリ・game_window バイナリ
└── xtask/              # ツール用クレート
    ├── Cargo.toml
    └── src/
        └── main.rs     # workspace-layout サブコマンド
```

### 3.2 xtask クレートの役割

| サブコマンド | 内容 |
|--------------|------|
| `workspace-layout` | `native/`, `lib/` 以下のソースをスキャンし、`WorkspaceLayout.md` をプロジェクトルートに生成 |

将来的にコード生成やその他のタスクを追加する場合も、同じ xtask 内にサブコマンドとして実装できます。

---

## 4. Workspace Layout ツール

### 4.1 目的

ファイル分散により **AI の精度が落ちる**のを防ぐため、コード全体の Path・Lines・Status・Summary を把握する Workspace Layout を自動生成します。開発者や AI がリポジトリ構造を素早く把握できるようにするのが狙いです。

### 4.2 出力形式

`WorkspaceLayout.md` は次のような Markdown テーブル形式で出力されます。Path は GitHub の該当ファイルへのリンクになります。

```markdown
# Workspace Layout（自動生成）

| Path | Lines | Status | Summary |
|------|-------|--------|---------|
| [native/game_native/src/lib.rs](https://github.com/FRICK-ELDY/elixir_rust/blob/main/native/game_native/src/lib.rs) | 1844 | 🔴 | NIF エントリ・ワールド型・物理ステップ・イベント・セーブをすべて含む |
...
```

### 4.3 Status 基準（行数による分割優先度）

| 行数 | Status | 記号 | 意味 |
|------|--------|------|------|
| 0–4 | 0 | ⚪ | 無評価 |
| 5–50 | 1 | 🟢 | OK、保持 |
| 51–100 | 2 | 🟡 | 様子見、早めに分割候補 |
| 101–200 | 3 | 🟠 | 分割推奨 |
| 201– | 4 | 🔴 | 最優先で分割 |

### 4.4 ファイルヘッダー規約（Path / Summary）

各ソースファイルの先頭に Path と Summary を記述し、AI が文脈を把握しやすくします。xtask がこのヘッダーをパースして WorkspaceLayout.md に反映します。

| 言語 | 例 |
|------|-----|
| Rust | `//! Path: ...` `//! Summary: ...` |
| Elixir | `# Path: ...` `# Summary: ...` |

詳細は [STEPS_RUST_LIB.md](../05_steps/01_engine/STEPS_RUST_LIB.md) の 1.6.1 節を参照してください。

---

## 5. 実行方法

### 5.1 コマンドライン

```powershell
# native/ に移動して実行
cd native
cargo run -p xtask -- workspace-layout
```

`cargo-xtask` をインストールしている場合は次のようにも実行できます。

```powershell
cd native
cargo xtask workspace-layout
```

### 5.2 bin スクリプト

プロジェクトルートから `bin/workspace_layout.bat` を実行することで、上記を簡易に実行できます。

```cmd
bin\workspace_layout.bat
```

### 5.3 出力先

`WorkspaceLayout.md` は**プロジェクトルート**（`mix.exs` と同じ階層）に生成されます。

---

## 6. 関連ドキュメント

| ドキュメント | 用途 |
|-------------|------|
| [STEPS_RUST_LIB.md](../05_steps/01_engine/STEPS_RUST_LIB.md) | Rust lib 分割・整理、Workspace Layout ツールの仕様（1.6.1） |
| [ELIXIR_RUST_DIVISION.md](./ELIXIR_RUST_DIVISION.md) | Elixir/Rust 役割分担方針 |
| [WHY_Rustler.md](./WHY_Rustler.md) | NIF 連携に Rustler を採用した理由 |
