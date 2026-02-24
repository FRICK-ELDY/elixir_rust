# なぜ wgpu を描画 API に選ぶのか

このドキュメントでは、GPU 描画ライブラリとして wgpu を採用した技術的根拠を説明します。ゲームの描画層における根幹となる選択です。

---

## 目次

1. [wgpu とは](#1-wgpu-とは)
2. [採用理由](#2-採用理由)
3. [本プロジェクトでの利用](#3-本プロジェクトでの利用)
4. [winit・egui との関係](#4-winitegui-との関係)
5. [関連ドキュメント](#5-関連ドキュメント)

---

## 1. wgpu とは

wgpu は、Vulkan / Metal / DX12 を抽象化したクロスプラットフォーム GPU 描画 API です。WebGPU 仕様に準拠した API を提供します。

- **バージョン**: wgpu 24（Cargo.toml）
- **役割**: スプライト描画、2D レンダリング、パーティクル、HUD
- **対応プラットフォーム**: Windows（DX12/Vulkan）、macOS（Metal）、Linux（Vulkan）
- **プロジェクト**: gfx-rs の一環、WebGPU 参照実装としても利用される

---

## 2. 採用理由

### 2.1 クロスプラットフォーム

Vulkan、Metal、DX12 を統一的に扱えるため、OS ごとの描画コードの分岐を最小化できます。同じシェーダー・パイプライン記述で各プラットフォームに対応します。

### 2.2 メンテナンスが活発

gfx-rs プロジェクトの一環として開発され、WebGPU の参照実装としても利用されています。wgpu 0.20 以降は API も安定してきました。Chrome、Firefox 等のブラウザ実装との親和性も高いです。

### 2.3 スプライト・2D に十分

本プロジェクトは 2D スプライトベースのゲームです。wgpu の機能で十分であり、Unity や Unreal のようなフル機能エンジンは過剰です。必要な機能（テクスチャサンプリング、ブレンディング、シンプルな 2D パイプライン）を直接制御できます。

### 2.4 代替との比較

| 手段 | クロスプラットフォーム | メンテナンス | GPU 活用 | 用途 |
|------|------------------------|--------------|----------|------|
| **wgpu** | 〇 | 活発 | 〇 | 2D/3D、WebGPU 準拠 |
| glutin/glium | 〇 | 低調 | 〇 | レガシー |
| SDL2 + ソフトウェア描画 | 〇 | 活発 | × | 単純な 2D、軽量 |
| DirectX / Metal 直接 | × | 各 OS 別 | 〇 | ネイティブ最適化 |
| minifb | 〇 | 活発 | × | フレームバッファ直接操作 |

---

## 3. 本プロジェクトでの利用

- **用途**: スプライトアトラス描画、パーティクル、HUD
- **連携**:
  - [pollster](./WHY_pollster.md) で `request_adapter` / `request_device` 等の非同期 API を同期実行
  - [bytemuck](./WHY_bytemuck.md) / [image](./WHY_image.md) でテクスチャデータのロード・変換
  - [winit](./WHY_winit.md) でウィンドウ・サーフェスを取得
- **egui**: デバッグ UI に `egui-wgpu` バックエンドを使用（[WHY_egui.md](./WHY_egui.md) 参照）

---

## 4. winit・egui との関係

```
winit（ウィンドウ・入力）→ wgpu（描画）→ フレームバッファ
                              ↑
                         egui-wgpu（デバッグ UI オーバーレイ）
```

- **winit**: ウィンドウ作成、キー・マウスイベント。wgpu の `create_surface` に渡す。詳細は [WHY_winit.md](./WHY_winit.md)
- **egui**: デバッグ用の即座モード GUI。wgpu で描画するため `egui-wgpu` を使用。詳細は [WHY_egui.md](./WHY_egui.md)

---

## 5. 関連ドキュメント

| ドキュメント | 用途 |
|-------------|------|
| [WHY_winit.md](./WHY_winit.md) | ウィンドウ・入力 |
| [WHY_egui.md](./WHY_egui.md) | デバッグ UI |
| [WHY_pollster.md](./WHY_pollster.md) | 非同期 API の同期実行 |
| [WHY_bytemuck.md](./WHY_bytemuck.md) | テクスチャデータの型変換 |
| [WHY_image.md](./WHY_image.md) | 画像読み込み |
| [WHY_RUST.md](../WHY_RUST.md) | Rust 採用の技術的根拠。依存関係一覧あり |
| [ELIXIR_RUST_DIVISION.md](../../ELIXIR_RUST_DIVISION.md) | Elixir/Rust 役割分担（描画は Rust 担当） |
