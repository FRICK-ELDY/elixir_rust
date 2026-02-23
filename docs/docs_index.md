# ドキュメント索引

**プロジェクト**: Elixir × Rust Survivor（ヴァンパイアサバイバーライクゲーム）  
**最終更新**: 2026-02-23

このディレクトリ内のドキュメントを用途別に整理した索引です。

---

## 目次

1. [入門・セットアップ](#1-入門セットアップ-01_setup)
2. [仕様・設計](#2-仕様設計-02_spec_design)
3. [技術選定の背景](#3-技術選定の背景-03_tech_decisions)
4. [実装ロードマップ](#4-実装ロードマップ-04_roadmap)
5. [ステップガイド（実装手順）](#5-ステップガイド実装手順-05_steps)
6. [システム設計・提案](#6-システム設計提案-06_system_design)
7. [プレゼンテーション](#7-プレゼンテーション-07_presentation)

---

## 1. 入門・セットアップ `01_setup/`

| ドキュメント | 内容 | 想定読者 |
|-------------|------|----------|
| [SETUP_ELIXIR.md](./01_setup/SETUP_ELIXIR.md) | Elixir / Erlang / rustler の環境構築手順（Windows 中心）。インストーラー・Chocolatey・Scoop など複数の方法を記載 | 開発開始前 |
| [SPEC.md](./01_setup/SPEC.md) | ゲーム仕様書。ゲームデザイン、技術アーキテクチャ、ECS 設計、NIF API、レンダリング、物理演算、パフォーマンス仕様など | 全体像把握時 |

---

## 2. 仕様・設計 `02_spec_design/`

| ドキュメント | 内容 | 想定読者 |
|-------------|------|----------|
| [ENGINE_ANALYSIS_REVISED.md](./02_spec_design/ENGINE_ANALYSIS_REVISED.md) | **ゲームエンジンの再評価版**。STEPS_PERF・PRIORITY_STEPS 導入後の弱みの対応状況、推奨改善方向の更新、総合評価 | 設計理解・評価時 |
| [ENGINE_ANALYSIS.md](./02_spec_design/ENGINE_ANALYSIS.md) | ゲームエンジンの強み・弱み分析（アーカイブ）。元の評価、他エンジンとの比較、ユースケース適性 | 過去分析の参照 |

---

## 3. 技術選定の背景 `03_tech_decisions/`

| ドキュメント | 内容 | 想定読者 |
|-------------|------|----------|
| [WHY_ELIXIR.md](./03_tech_decisions/WHY_ELIXIR.md) | ゲームロジック層に Elixir（BEAM VM）を採用した理由。BEAM の設計思想、ゲームループ適合性、並行性、耐障害性、Rust との役割分担 | アーキテクチャ理解時 |
| [ELIXIR_RUST_DIVISION.md](./03_tech_decisions/ELIXIR_RUST_DIVISION.md) | **Elixir/Rust 役割分担方針**。「苦手なものは Rust に投げる」。タイミング精度、将来の拡張、スコープ外・サポートしない項目 | アーキテクチャ理解時 |
| [WHY_RAYON.md](./03_tech_decisions/WHY_RAYON.md) | Chase AI の並列化に rayon を採用した理由。Work-Stealing、SoA との相乗効果、実測結果、他ライブラリとの比較 | パフォーマンス理解時 |

---

## 4. 実装ロードマップ `04_roadmap/`

| ドキュメント | 内容 | 想定読者 |
|-------------|------|----------|
| [PRIORITY_STEPS.md](./04_roadmap/PRIORITY_STEPS.md) | **「何から手をつけるか」の優先度ロードマップ**。パフォーマンス最優先 → 汎用化 → 品質の順。P1〜P7、G1〜G3、Q1〜Q2 の全体マップ。詳細手順は [STEPS_PERF.md](./05_steps/STEPS_PERF.md) を参照 | 実装計画立案時 |
| [NEXT_STEPS.md](./04_roadmap/NEXT_STEPS.md) | **次の Step 提案**（Step 32〜40）。ヴァンサバ以外のゲームでも使える汎用エンジン化。Game インターフェース、ゲーム分離、プラグイン化 | 汎用化計画立案時 |

---

## 5. ステップガイド（実装手順）`05_steps/`

ゲームは **Step 1** から順に実装していく構成。段階ごとに別ドキュメントで詳細を記述。

### 5.1 基礎実装（Step 1〜15）

| ドキュメント | 内容 | ステップ範囲 |
|-------------|------|--------------|
| [STEPS.md](./05_steps/STEPS.md) | 環境構築からゲームオーバー・リスタートまで。Rust クレート、NIF 連携、ゲームループ、プレイヤー・敵・武器・UI・レベルアップ | Step 1 〜 Step 15 |

### 5.2 クオリティアップ（Step 16〜25）

| ドキュメント | 内容 | ステップ範囲 |
|-------------|------|--------------|
| [STEPS_QUALITY.md](./05_steps/STEPS_QUALITY.md) | ヒットエフェクト、武器強化、敵タイプ追加、アイテムドロップ、カメラ、BGM/SE、スプライトアニメ、ボス、バランス調整 | Step 16 〜 Step 25 |

### 5.3 パフォーマンス改善（Step 26〜31）

| ドキュメント | 内容 | ステップ範囲 |
|-------------|------|--------------|
| [STEPS_PERF.md](./05_steps/STEPS_PERF.md) | **詳細な実装手順**。イベントバス、ETS キャッシュ、フリーリスト、Spatial Hash 最近接、RwLock、Telemetry、SIMD AI。[PRIORITY_STEPS.md](./04_roadmap/PRIORITY_STEPS.md) 準拠の推奨順序あり | Step 26 〜 Step 31 |
| [STEPS_PERFORMANCE.md](./05_steps/STEPS_PERFORMANCE.md) | **現状分析と改善提案**。ボトルネックの洗い出し、課題一覧、具体的な改善手法の解説。実装手順よりも「なぜ」「何を」に焦点 | 分析・設計参考 |

### 5.4 機能拡張（Step 41〜44）

| ドキュメント | 内容 | ステップ範囲 |
|-------------|------|--------------|
| [STEPS_MAP_SAVE_MULTI_DEBUG.md](./05_steps/STEPS_MAP_SAVE_MULTI_DEBUG.md) | **マップ・セーブ・マルチプレイ・デバッグ支援**。障害物・壁・タイル、ゲーム状態永続化、マルチプレイ基盤、NIF デバッグ容易化 | Step 41 〜 Step 44 |

---

## 6. システム設計・提案 `06_system_design/`

| ドキュメント | 内容 | 状態 |
|-------------|------|------|
| [ASSET_MANAGEMENT.md](./06_system_design/ASSET_MANAGEMENT.md) | G3: アセット管理システム設計。AssetId、パスマッピング、Elixir/Rust の責務分離、実行時ロード | PRIORITY_STEPS G3 関連 |
| [REFACTOR_PROPOSAL.md](./06_system_design/REFACTOR_PROPOSAL.md) | `lib.rs` / `main.rs` の責務整理。`renderer` を lib 側に移動する提案。実施タイミング: Step 15 完了後 | 提案中 |

---

## 7. プレゼンテーション `07_presentation/`

| ドキュメント | 内容 | 想定読者 |
|-------------|------|----------|
| [PRESENTATION.md](./07_presentation/PRESENTATION.md) | 技術発表資料。「Elixir × Rust でなぜ強くなるか」の全体像、アーキテクチャ図、デモ結果、今後の展望 | 外部発表・共有用 |

---

## フォルダ構成

```
docs/
├── docs_index.md          ← 本ファイル（索引）
├── 01_setup/              入門・セットアップ
│   ├── SETUP_ELIXIR.md
│   └── SPEC.md
├── 02_spec_design/        仕様・設計
│   ├── ENGINE_ANALYSIS_REVISED.md
│   └── ENGINE_ANALYSIS.md（アーカイブ）
├── 03_tech_decisions/     技術選定の背景
│   ├── WHY_ELIXIR.md
│   ├── ELIXIR_RUST_DIVISION.md
│   └── WHY_RAYON.md
├── 04_roadmap/            実装ロードマップ
│   ├── PRIORITY_STEPS.md
│   └── NEXT_STEPS.md
├── 05_steps/              ステップガイド
│   ├── STEPS.md
│   ├── STEPS_QUALITY.md
│   ├── STEPS_PERF.md
│   ├── STEPS_PERFORMANCE.md
│   └── STEPS_MAP_SAVE_MULTI_DEBUG.md
├── 06_system_design/      システム設計・提案
│   ├── ASSET_MANAGEMENT.md
│   └── REFACTOR_PROPOSAL.md
└── 07_presentation/       プレゼンテーション
    └── PRESENTATION.md
```

---

## ドキュメント間の参照関係

```
01_setup/SPEC.md（仕様）
    ↓
05_steps/STEPS.md（Step 1-15）
    ↓
05_steps/STEPS_QUALITY.md（Step 16-25）
     ↓
02_spec_design/ENGINE_ANALYSIS_REVISED.md（再評価）← ENGINE_ANALYSIS.md（アーカイブ）
    ↓
04_roadmap/PRIORITY_STEPS.md（優先度マップ）
    ├── 05_steps/STEPS_PERF.md（Step 26-31 詳細手順）
    ├── 05_steps/STEPS_PERFORMANCE.md（分析・提案）
    └── 06_system_design/ASSET_MANAGEMENT.md（G3 設計）
```

---

## クイックリファレンス

| やりたいこと | 参照先 |
|-------------|--------|
| 環境を整えたい | [SETUP_ELIXIR.md](./01_setup/SETUP_ELIXIR.md) |
| 仕様を把握したい | [SPEC.md](./01_setup/SPEC.md) |
| ゼロから実装したい | [STEPS.md](./05_steps/STEPS.md) → [STEPS_QUALITY.md](./05_steps/STEPS_QUALITY.md) |
| 改善の優先度を知りたい | [PRIORITY_STEPS.md](./04_roadmap/PRIORITY_STEPS.md) |
| 汎用ゲームエンジン化の次の Step を知りたい | [NEXT_STEPS.md](./04_roadmap/NEXT_STEPS.md) |
| マップ・セーブ・マルチプレイ・デバッグを実装したい | [STEPS_MAP_SAVE_MULTI_DEBUG.md](./05_steps/STEPS_MAP_SAVE_MULTI_DEBUG.md) |
| パフォーマンス改善を実装したい | [STEPS_PERF.md](./05_steps/STEPS_PERF.md) |
| なぜこの構成なのか理解したい | [WHY_ELIXIR.md](./03_tech_decisions/WHY_ELIXIR.md), [ELIXIR_RUST_DIVISION.md](./03_tech_decisions/ELIXIR_RUST_DIVISION.md), [WHY_RAYON.md](./03_tech_decisions/WHY_RAYON.md) |
| エンジンの評価・比較を知りたい | [ENGINE_ANALYSIS_REVISED.md](./02_spec_design/ENGINE_ANALYSIS_REVISED.md) |
| 発表用の資料が欲しい | [PRESENTATION.md](./07_presentation/PRESENTATION.md) |
