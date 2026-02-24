# なぜ winit をウィンドウ・入力に選ぶのか

このドキュメントでは、ウィンドウ管理・キーボード・マウス入力に winit を採用した技術的根拠を説明します。wgpu との組み合わせで描画の基盤を構成します。

---

## 目次

1. [winit とは](#1-winit-とは)
2. [採用理由](#2-採用理由)
3. [wgpu との連携](#3-wgpu-との連携)
4. [本プロジェクトでの利用](#4-本プロジェクトでの利用)
5. [関連ドキュメント](#5-関連ドキュメント)

---

## 1. winit とは

winit は、クロスプラットフォームのウィンドウ・イベント API です。Rust のゲーム・GUI アプリケーションで広く利用されています。

- **バージョン**: winit 0.30.12（Cargo.toml）
- **役割**: ウィンドウ作成、リサイズ、キーボード・マウスイベントの取得
- **対応プラットフォーム**: Windows、macOS、Linux、Wayland、Web（wasm）

---

## 2. 採用理由

### 2.1 wgpu との相性

wgpu は winit の `Window` から `Surface` を作成する公式サポートがあります。`Instance::create_surface` に `winit::window::Window` を渡すだけで、各 OS に適した描画サーフェスを取得できます。

### 2.2 ゲーム開発での実績

bevy、wgpu サンプル、多くの Rust ゲーム・ツールが winit を採用しています。イベントループの設計、入力の扱いがゲーム用途に適しています。

### 2.3 クロスプラットフォーム

Windows（Win32）、macOS（Cocoa）、Linux（X11/Wayland）で同じ API で動作します。将来的な Web 出力（wasm）も winit がサポートしています。

### 2.4 イベント駆動

`EventLoop` によるイベント駆動モデルは、ゲームループと相性が良いです。キー押下・マウス移動・ウィンドウリサイズを適切に処理できます。

---

## 3. wgpu との連携

```
EventLoop::run() 
  → Window 作成
  → wgpu::Instance::create_surface(window) で Surface 取得
  → 各フレーム: surface.get_current_texture() → 描画 → present()
```

本プロジェクトでは、Rust 側のゲームループが wgpu の描画を主導し、winit のイベントループはウィンドウ管理と入力イベントの収集に使われます。入力は Elixir 側の `Engine.InputHandler` にも渡されます（game_window バイナリの場合）。

---

## 4. 本プロジェクトでの利用

- **用途**: ウィンドウ作成、キーボード（WASD、矢印キー等）、マウス入力
- **連携**: wgpu の Surface 作成、egui の `egui-winit` によるイベント処理
- **バイナリ**: `game_window` が winit のイベントループを駆動

---

## 5. 関連ドキュメント

| ドキュメント | 用途 |
|-------------|------|
| [WHY_wgpu.md](./WHY_wgpu.md) | wgpu の選定理由（描画 API） |
| [WHY_egui.md](./WHY_egui.md) | egui（egui-winit 連携）の選定理由 |
| [WHY_RUST.md](../WHY_RUST.md) | Rust 採用の技術的根拠 |
| [ELIXIR_RUST_DIVISION.md](../../ELIXIR_RUST_DIVISION.md) | 入力の役割分担（Rust がポーリング、Elixir が状態管理） |
