# なぜ pollster を使うのか

このドキュメントでは、非同期 API の同期実行に pollster を採用した技術的根拠を説明します。wgpu の初期化と密接に連携します。

---

## 目次

1. [pollster とは](#1-pollster-とは)
2. [採用理由](#2-採用理由)
3. [本プロジェクトでの利用](#3-本プロジェクトでの利用)
4. [関連ドキュメント](#4-関連ドキュメント)

---

## 1. pollster とは

pollster は、Rust の非同期処理を**同期的にブロック**して実行するライブラリです。

- **バージョン**: pollster 0.3（Cargo.toml）
- **役割**: `block_on` で async 関数を同期実行
- **用途**: wgpu の `request_adapter`、`request_device` 等

---

## 2. 採用理由

### 2.1 wgpu の非同期 API

wgpu の初期化は非同期です。`Instance::request_adapter`、`Adapter::request_device` などは `Future` を返します。ゲームの起動時は同期的に初期化を完了したいため、`block_on` で待機します。

### 2.2 軽量・単純

tokio や async-std のようなランタイムは不要です。ポーリングベースで `Future` を完了まで実行するだけのシンプルな実装です。ゲームのイベントループと競合しません。

### 2.3 依存が少ない

ランタイムを持たないため、ビルド時間・バイナリサイズへの影響が小さいです。wgpu サンプルでも標準的に使用されています。

---

## 3. 本プロジェクトでの利用

```rust
// wgpu 初期化時
let adapter = pollster::block_on(instance.request_adapter(&desc))?;
let (device, queue) = pollster::block_on(adapter.request_device(&device_desc, None))?;
```

ゲーム起動時の一度きりの初期化で使用し、実行時ループには影響しません。

---

## 4. 関連ドキュメント

| ドキュメント | 用途 |
|-------------|------|
| [WHY_wgpu.md](./WHY_wgpu.md) | wgpu の選定理由（pollster の主な利用先） |
| [WHY_RUST.md](../WHY_RUST.md) | Rust 採用の技術的根拠 |
