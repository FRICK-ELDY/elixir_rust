# 1.7 描画統合（game_window → game_native）

**所属**: [STEPS_ALL.md](../STEPS_ALL.md) 1章 エンジン構築 の 1.7 節。

**目的**: game_window バイナリを廃止し、renderer / winit / wgpu を game_native に統合する。NIF が描画スレッドを spawn し、`iex -S mix` 単一プロセスで wgpu 描画を実行する。

**前提**: 1.6 Rust lib 分割・整理が完了していること。

---

## 概要

| 項目 | 内容 |
|------|------|
| **アーキテクチャ** | NIF が描画スレッドを spawn。同一プロセス（iex -S mix）内で winit のイベントループ + wgpu 描画を実行 |
| **描画対象** | NIF 内の GameWorld（get_frame_metadata 等）と Elixir 側のシーン・UI 状態 |
| **状態管理** | 当面は案 C（GameWorld は Rust 側で保持）。将来案 A（Elixir で保持）をオーバーヘッド確認のため検証 |
| **ビルド対象** | まずは Windows。将来的にクロスビルド対応 |

---

## 項（今後決める）

項の詳細は今後の設計・実装に合わせて追記する。

---

## 関連ドキュメント

| ドキュメント | 用途 |
|-------------|------|
| [FOLDER_CONNECTIONS.md](../../06_system_design/FOLDER_CONNECTIONS.md) | フォルダ接続関係 |
| [STEPS_RUST_LIB.md](./STEPS_RUST_LIB.md) | 1.6 のクレート構成 |
