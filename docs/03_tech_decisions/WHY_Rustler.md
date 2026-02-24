# なぜ Rustler を NIF 連携に選ぶのか

このドキュメントでは、Elixir と Rust を結合する NIF（Native Implemented Functions）ライブラリとして Rustler を採用した技術的根拠を説明します。Elixir 側・Rust 側の両観点を統合して記述します。

---

## 目次

1. [Rustler とは](#1-rustler-とは)
2. [採用理由](#2-採用理由)
3. [Elixir 側の利用](#3-elixir-側の利用)
4. [Rust 側の利用](#4-rust-側の利用)
5. [本プロジェクトでの構成](#5-本プロジェクトでの構成)
6. [関連ドキュメント](#6-関連ドキュメント)

---

## 1. Rustler とは

Rustler は、Rust で書いたネイティブ関数を Elixir から呼び出せるようにするライブラリです。

- **Elixir パッケージ**: rustler ~> 0.34（mix.exs）
- **Rust クレート**: rustler 0.34（Cargo.toml）
- **役割**: Elixir の `:erlang.load_nif/2` をラップし、型変換・ResourceArc・dirty_cpu スケジューラ対応を提供
- **エコシステム**: Elixir と Rust の連携において事実上の標準

---

## 2. 採用理由

### 2.1 透過的な呼び出し

Rust の関数が Elixir から**通常の関数呼び出しと同じ構文**で呼べます。

```elixir
# Elixir から見ると普通の関数呼び出し
Engine.spawn_enemies(world_ref, :slime, 5)
App.NifBridge.get_frame_metadata(world_ref)
```

### 2.2 ResourceArc によるゼロコピー連携

ゲームワールドの大量データを毎フレーム Elixir-Rust 間でコピーするのは非効率です。Rustler の `ResourceArc` を使うと、Rust のデータを Elixir から**参照（不透明なハンドル）**として扱えます。

```
従来のアプローチ（非効率）:
  Elixir → [データをシリアライズ] → Rust → [処理] → [デシリアライズ] → Elixir

ResourceArc アプローチ（効率的）:
  Elixir → [world_ref（8バイトの参照）] → Rust → [処理] → [必要なデータのみ返却] → Elixir
```

### 2.3 dirty_cpu スケジューラ対応

長時間 CPU を使用する NIF は `DirtyCpu` スケジューラで実行できます。BEAM の通常スケジューラをブロックせず、ゲームループの安定性を保ちます。

### 2.4 他言語との比較

| 手段 | Elixir 連携 | 型安全性 | メモリ安全性 |
|------|-------------|----------|--------------|
| **Rustler** | ネイティブサポート | コンパイル時 | Rust 保証 |
| C NIF（手動） | load_nif/2 を自前実装 | 手動 | 手動 |
| Port | プロセス間通信 | シリアライズ必要 | 分離 |
| Go（CGo） | 複雑、BEAM 連携は非標準 | 限定的 | Go の範囲内 |

---

## 3. Elixir 側の利用

- **App.NifBridge**: `use Rustler` により NIF をロード。Rustler で定義した関数の Elixir 側インターフェース
- **Engine モジュール**: ゲームからは `Engine` 経由で利用（`App.NifBridge` をラップ）
- **呼び出し例**: `Engine.spawn_enemies(world_ref, :slime, 5)` → 内部で `App.NifBridge.spawn_enemies` を呼び出し

---

## 4. Rust 側の利用

- **NIF 関数のエクスポート**: `#[rustler::nif]` で Elixir から呼び出せる関数を定義
- **ResourceArc**: ゲームワールド（`GameWorld`）等の大きなデータを参照として Elixir に渡す
- **DirtyCpu**: 長時間実行する NIF は `#[rustler::nif(schedule = "DirtyCpu")]` で BEAM をブロックしない

---

## 5. 本プロジェクトでの構成

- **主な NIF**: `create_world`、`spawn_enemies`、`get_frame_metadata`、`set_player_input`、`start_rust_game_loop` など
- **用途**: 物理演算、描画データ取得、ゲーム状態の読み書き

物理演算は Rust 内のゲームループで実行され、Elixir は `{:frame_events, events}` を受信してイベント駆動で応答します。詳細は [WHY_ELIXIR.md](./WHY_ELIXIR/WHY_ELIXIR.md) §6 および [WHY_RUST.md](./WHY_RUST/WHY_RUST.md) を参照してください。

---

## 6. 関連ドキュメント

| ドキュメント | 用途 |
|-------------|------|
| [WHY_ELIXIR/WHY_ELIXIR.md](./WHY_ELIXIR/WHY_ELIXIR.md) | Elixir 採用の技術的根拠、Rust との役割分担 |
| [WHY_RUST/WHY_RUST.md](./WHY_RUST/WHY_RUST.md) | Rust 採用の技術的根拠 |
| [ELIXIR_RUST_DIVISION.md](./ELIXIR_RUST_DIVISION.md) | Elixir/Rust 役割分担方針 |
