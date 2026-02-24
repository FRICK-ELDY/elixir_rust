# なぜ Rust をゲームの実行層に選ぶのか

このドキュメントでは、ヴァンパイアサバイバーライクゲームの物理演算・描画・音声・入力層に Rust を採用した技術的根拠を説明します。

### 依存関係の詳細と一覧

| パッケージ | 用途 | 詳細 |
|------------|------|------|
| rustler | Elixir NIF 連携 | [WHY_Rustler.md](../WHY_Rustler.md) |
| wgpu 系 | GPU 描画・ウィンドウ・テクスチャ | [WHY_WGPU/](./WHY_WGPU/WHY_wgpu.md) |
| rayon | Chase AI 並列化 | [WHY_RAYON.md](./WHY_RAYON.md) |
| rustc-hash | Spatial Hash（衝突判定） | [WHY_PHYSICS/WHY_rustc_hash.md](./WHY_PHYSICS/WHY_rustc_hash.md) |
| rodio | 音声再生（BGM/SE） | [WHY_AUDIO/WHY_rodio.md](./WHY_AUDIO/WHY_rodio.md) |
| serde, bincode | セーブ/ロード | [WHY_SERIALIZE/](./WHY_SERIALIZE/WHY_serde.md) |
| log, env_logger | NIF デバッグ（RUST_LOG、パニック時のバックトレース） | [STEPS_EXTENSION.md §6 Step 45](../../05_steps/STEPS_EXTENSION.md#6-step-45-デバッグ支援nif) |

---

## 目次

1. [Rust が担う役割](#1-rust-が担う役割)
2. [採用理由：性能と制御](#2-採用理由性能と制御)
3. [Elixir との役割分担](#3-elixir-との役割分担)
4. [他言語との比較](#4-他言語との比較)
5. [トレードオフと注意点](#5-トレードオフと注意点)

---

## 1. Rust が担う役割

本プロジェクトにおいて Rust は以下を担当します。

| レイヤー | 内容 |
|----------|------|
| **物理演算** | 敵・弾丸の位置更新、Chase AI、Spatial Hash、衝突判定 |
| **描画** | wgpu による GPU レンダリング、スプライト描画 |
| **音声** | rodio による BGM/SE 再生 |
| **入力** | winit によるウィンドウ・入力ポーリング |
| **空間構造** | SoA レイアウト、フリーリスト、SIMD 最適化 |

Elixir が「司令塔」であるのに対し、Rust は「**実行エンジン**」として毎フレームの重い処理を担います。

---

## 2. 採用理由：性能と制御

### 2.1 ゼロコスト抽象化

Rust の抽象化（ジェネリクス、トレイト、イテレータ）はコンパイル時に展開され、**実行時オーバーヘッドがゼロ**です。ゲームループ内で 16ms を切り崩す必要があるため、この特性は決定的です。

### 2.2 メモリレイアウト制御（SoA）

敵 5000 体以上のデータを **SoA（Structure of Arrays）** で保持することで、キャッシュ効率を最大化し、並列化（rayon）と組み合わせてスループットを上げています。

```
SoA: positions_x, positions_y, velocities_x, velocities_y, speeds, alive
     ↑ 連続メモリでキャッシュヒット率が高い
```

詳細は [WHY_RAYON.md](./WHY_RAYON.md) を参照。

### 2.3 低レイテンシ・厳密なタイミング

- **GC なし**: 数値計算・物理演算の途中で GC が割り込まない
- ** deterministic な計算**: 同じ入力から同じ出力が保証され、将来のリプレイ・ロールバックに有利
- **直接的な制御**: CPU・メモリ・GPU を細かく制御可能

[ELIXIR_RUST_DIVISION.md](../ELIXIR_RUST_DIVISION.md) の「タイミングクリティカルなパスは Rust 内に閉じる」方針に基づいています。

### 2.4 GPU 操作（wgpu）

wgpu は Vulkan/Metal/DX12 の抽象化レイヤーとして、クロスプラットフォームで GPU を扱えます。Elixir 側で直接 GPU を触ることは現実的ではないため、Rust がこの責務を担います。

---

## 3. Elixir との役割分担

### 「苦手なものは Rust に投げる」

| 観点 | Elixir が苦手 | Rust が得意 |
|------|---------------|-------------|
| 低レイテンシ | GC の影響あり | GC なし |
| 厳密なタイミング | Process.send_after のジッター | 固定間隔で実行可能 |
| 数値計算 | オーバーヘッド | ゼロコスト抽象化 |
| GPU 操作 | 不可 | wgpu |
| メモリレイアウト | 制御困難 | SoA 等で完全制御 |

逆に、**並行プロセス管理・耐障害性・分散**は Elixir が得意であり、Rust に任せる必要はありません。

詳細は [WHY_ELIXIR](../WHY_ELIXIR/WHY_ELIXIR.md) §6 および [ELIXIR_RUST_DIVISION.md](../ELIXIR_RUST_DIVISION.md) を参照。

---

## 4. 他言語との比較

### ゲーム実行層の候補

| 言語 | 利点 | 課題 |
|------|------|------|
| **Rust** | ゼロコスト、GC なし、wgpu、Rustler で Elixir と自然に連携 | 学習コスト、ビルド時間 |
| C/C++ | 最高速、実績豊富 | メモリ安全でない、Elixir 連携が煩雑 |
| Go | シンプル、並行性 | GC あり、wgpu 等の GPU ライブラリが弱い |
| Zig | C 互換、メモリ制御 | エコシステムが未成熟、Rustler 相当の連携がない |

**Rust を選んだ決定的理由**は、**Rustler による Elixir との透過的な NIF 連携**と、**性能・安全性・エコシステムのバランス**です。

---

## 5. トレードオフと注意点

### 学習コスト

所有権・借用・ライフタイムの概念は初学者には難しいです。ただし、ゲーム実行層は比較的閉じたレイヤーであり、既存の wgpu・rayon などのパターンに従うことで習得コストを抑えられます。

### ビルド時間

Rust のコンパイルは C++ より速い場合が多いですが、リリースビルドは時間がかかります。開発時は `opt-level = 1` やインクリメンタルビルドで軽減しています。

### NIF の責任

Rust の NIF がパニックすると BEAM VM ごとクラッシュするリスクがあります。Rustler 0.29+ では `catch_unwind` でラップされますが、unsafe ブロック内の未定義動作は防げません。対策として、unsafe を最小限にし、テスト・プロファイリングを定期的に実施します。加えて **Step 45（デバッグ支援）** で、デバッグビルド時のパニックフック（`RUST_BACKTRACE=1` でバックトレース表示）と `log` / `env_logger`（`RUST_LOG` で Rust 側ログ出力）を導入し、Elixir/Rust 境界のバグ追跡を容易にしています。

---

## まとめ

Rust は「**ゲームの実行エンジン**」として、物理演算・描画・音声・入力を担います。

- **ゼロコスト抽象化**と **SoA レイアウト**で、5000 体以上の敵を 60fps で処理
- **Rustler NIF** により Elixir から透過的に呼び出せる
- **wgpu** でクロスプラットフォーム GPU 描画
- **Elixir と役割分担**し、タイミングクリティカルな処理を Rust 内に閉じる

Elixir の生産性と Rust の性能を両立する、現時点での最良のアーキテクチャの一つです。

---

## 関連ドキュメント

| ドキュメント | 用途 |
|-------------|------|
| [WHY_WGPU/](./WHY_WGPU/WHY_wgpu.md) | wgpu、winit、egui、pollster、bytemuck、image |
| [WHY_AUDIO/](./WHY_AUDIO/WHY_rodio.md) | rodio（音声） |
| [WHY_SERIALIZE/](./WHY_SERIALIZE/WHY_serde.md) | serde、bincode（セーブ/ロード） |
| [WHY_PHYSICS/](./WHY_PHYSICS/WHY_rustc_hash.md) | rustc-hash（Spatial Hash） |
| [WHY_Rustler.md](../WHY_Rustler.md) | Rustler（NIF 連携） |
| [WHY_RAYON.md](./WHY_RAYON.md) | rayon |
| [WHY_ELIXIR/WHY_ELIXIR.md](../WHY_ELIXIR/WHY_ELIXIR.md) | Elixir 採用の技術的根拠 |
| [ELIXIR_RUST_DIVISION.md](../ELIXIR_RUST_DIVISION.md) | Elixir/Rust 役割分担方針 |
| [STEPS_EXTENSION.md §6 Step 45](../../05_steps/STEPS_EXTENSION.md#6-step-45-デバッグ支援nif) | log / env_logger（NIF デバッグ支援） |
