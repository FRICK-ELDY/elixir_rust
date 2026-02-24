# 実装ステップ一覧（Step 1〜47）

**目的**: 各フェーズのステップを一つのファイルにまとめ、全体像と推奨順序を把握しやすくする。  
**詳細な手順・コード例**は各フェーズの元ドキュメントを参照すること。

**ネーミング規則**: すべて「Step N: メインタイトル（補足）」の形で統一。メインタイトルは名詞・名詞句で表記。

---

## 1. 全体ロードマップ

| フェーズ | Step | 内容 |
|----------|------|------|
| **基礎** | 1〜15 | 環境構築〜ゲームオーバー・リスタートまで「動くゲーム」 |
| **クオリティ** | 16〜25 | ヒットエフェクト〜ボス・バランスで「楽しめるゲーム」 |
| **パフォーマンス** | 26〜31 | イベントバス・ETS・フリーリスト・Spatial Hash・Telemetry・SIMD |
| **汎用化** | 32〜39 | Game インターフェース・シーン汎用化・ゲーム分離・2 つ目のゲーム土台 |
| **拡張** | 40〜47 | ゲームループの Rust 移行・マップ・セーブ・マルチ・デバッグ・GameEvents リネーム・SPEC 未実装コンテンツ |
| **3D・三人称FPS** | 48〜54 | WGPU 3D 基盤・カメラ・メッシュ・プレイヤー制御・射撃・敵AI・UI（[STEPS_3D.md](./STEPS_3D.md)） |
| **Slot・コンポーネント** | 55〜61 | シーングラフ（Slot）と Component を Elixir で管理・スナップショットで Rust 描画・シリアライズ・Prefab・エディタ基盤（[STEPS_SLOT_COMPONENT.md](./STEPS_SLOT_COMPONENT.md)） |

---

## 2. Step 1〜15: 基礎実装

| Step | 目標 | 備考 |
|------|------|------|
| 1 | 環境構築 | |
| 2 | ウィンドウ表示（Rust クレート・winit） | |
| 3 | wgpu 初期化・単色クリア | |
| 4 | スプライト 1 枚描画 | |
| 5 | インスタンシング描画（100 体） | |
| 6 | NIF 連携（Elixir + Rustler） | |
| 7 | ゲームループ（GenServer 60 Hz） | ※BEAM スケジューラの特性上、高精度なタイマー実行が保証されないため、Step 41 で Rust に移行 |
| 8 | プレイヤー移動 | |
| 9 | 敵スポーン・追跡 AI | |
| 10 | 衝突判定（Spatial Hash） | |
| 11 | 武器・弾丸システム | |
| 12 | 大規模スポーン・最適化（5000 体） | |
| 13 | UI（HP・スコア・タイマー） | |
| 14 | レベルアップ・武器選択 | |
| 15 | ゲームオーバー・リスタート | |

**詳細**: [STEPS_BASE.md](./STEPS_BASE.md)

---

## 3. Step 16〜25: クオリティアップ

| Step | 目標 |
|------|------|
| 16 | ヒットエフェクト（パーティクル） |
| 17 | 武器レベルアップ（重ね強化） |
| 18 | 複数敵タイプ |
| 19 | アイテムドロップ（回復・磁石） |
| 20 | カメラシステム（プレイヤー追従） |
| 21 | 武器追加（Whip / Fireball / Lightning） |
| 22 | BGM・SE（rodio） |
| 23 | スプライトアニメーション |
| 24 | ボスエネミー |
| 25 | バランス調整・ポリッシュ |

**詳細**: [STEPS_QUALITY.md](./STEPS_QUALITY.md)

---

## 4. Step 26〜31: パフォーマンス改善

| Step | 目標 | 備考 |
|------|------|------|
| 26 | イベントバス | フレームイベントを Elixir に配信 |
| 27 | ETS キャッシュ・入力ポーリング | プロセス間通信の最適化 |
| 28 | フリーリスト（スポーン O(1)） | P3 |
| 29 | Spatial Hash 最近接・RwLock | AI 高速化（P1, P2） |
| 30 | Telemetry 計測基盤 | 観測可能性（P7） |
| 31 | SIMD AI 高速化（オプション） | P4 |

**推奨順（パフォーマンス優先）**: 29 → 28 → 26 → 27 → 30 → 31  
**詳細**: [STEPS_PERF.md](./STEPS_PERF.md)  
**分析・課題整理**: [STEPS_PERFORMANCE_ANALYSIS.md](./STEPS_PERFORMANCE_ANALYSIS.md)

---

## 5. Step 32〜39: 汎用ゲームエンジン化

| Step | 目標 |
|------|------|
| 32 | Game インターフェース設計 |
| 33 | render_type 汎用化 |
| 34 | ゲーム切替（config） |
| 35 | ゲーム分離（vampire_survivor） |
| 36 | Game behaviour 実装 |
| 37 | エンジン API 安定化 |
| 38 | entity_registry（データ駆動） |
| 39 | ゲーム別アセットパス |
| 40 | 2 つ目のゲーム（ミニマル実装） |

**詳細**: [STEPS_GENERALIZATION.md](./STEPS_GENERALIZATION.md)（Step 32〜40）

---

## 6. Step 41〜47: マップ・セーブ・マルチ・デバッグ・リネーム・SPEC コンテンツ

| Step | 目標 | 備考 |
|------|------|------|
| 41 | ゲームループの Rust 移行（高精度 60 Hz） | Step 7 のゲームループを Rust 側に移行（BEAM スケジューラ上では高精度なタイマー実行が保証されないため） |
| 42 | マップ・障害物システム | |
| 43 | セーブ・ロード | |
| 44 | マルチプレイ基盤（ルーム管理） | |
| 45 | デバッグ支援（NIF） | |
| 46 | GameLoop を GameEvents にリネーム | Elixir 側の役割（frame_events 受信・フェーズ管理）に合わせてモジュール名を変更 |
| 47 | SPEC 未実装コンテンツ（Skeleton / Ghost / Garlic / 壁すり抜け） | 敵 Skeleton・Ghost、武器 Garlic、Ghost の障害物すり抜けを実装 |

**推奨順序**: 41 → 45 → 42 → 43 → 44 → 46（42 と 43 は並行可）。47 は 42 完了後が望ましい。  
**詳細**: [STEPS_EXTENSION.md](./STEPS_EXTENSION.md)

---

## 7. Step 48〜54: 3D・三人称 FPS

| Step | 目標 | 備考 |
|------|------|------|
| 48 | 3D レンダリング基盤 | wgpu で 3D パイプライン・深度バッファ・頂点/法線/UV |
| 49 | 3D カメラ・三人称視点 | View/Projection 行列・カメラ追従 |
| 50 | 3D メッシュ描画・アトラス流用 | 既存スプライトアトラスを 3D テクスチャに |
| 51 | 三人称プレイヤー制御 | 移動・カメラ追従・照準（WASD・マウス） |
| 52 | 射撃・弾丸・レイキャスト | FPS 武器・ヒット判定 |
| 53 | 敵の 3D スポーン・AI・衝突 | 敵配置・Chase AI・ダメージ・経験値 |
| 54 | UI・アセット流用・ポリッシュ | HUD・BGM/SE 流用・ゲーム選択統合 |

**目的**: WGPU 対応プラットフォームで 3D を動かし、[ENGINE_STRENGTHS_WEAKNESSES.md](../02_spec_design/ENGINE_STRENGTHS_WEAKNESSES.md) の 3D ゲーム適性を上げる。アセットはヴァンパイアサバイバーズ系画像を流用。  
**詳細**: [STEPS_3D.md](./STEPS_3D.md)

---

## 8. Step 55〜61: Slot・コンポーネント

| Step | 目標 | 備考 |
|------|------|------|
| 55 | Slot（transform 階層）のデータ構造 | Elixir で id / parent_id / local_transform を管理 |
| 56 | コンポーネント型・レジストリと Slot への付与 | Camera, Player, Enemy, Mesh 等の型と Slot への付与 |
| 57 | ワールド行列計算・シーンスナップショット・Rust 連携 | 毎フレーム Elixir → Rust にスナップショットを渡して描画 |
| 58 | 物理結果の Rust → Elixir 反映（Slot 同期） | 物理ステップの結果で Slot の位置を更新 |
| 59 | シーンシリアライズ・保存/ロード | シーンファイル形式・バージョン・ロード |
| 60 | Prefab とインスタンス | 再利用可能な Slot サブツリー・インスタンス化とオーバーライド |
| 61 | ビジュアルエディタ向け基盤 | コンポーネントスキーマ公開・選択・Undo の検討と最小実装 |

**目的**: Step 48〜54 完了後、Slot（transform 階層）＋ Component を Elixir で管理し、Rust はシーンスナップショットで描画する設計に移行。ビジュアルエディタを見据えた基盤を整える。  
**詳細**: [STEPS_SLOT_COMPONENT.md](./STEPS_SLOT_COMPONENT.md)

---

## 9. 依存関係（概要）

- **1〜15**: 直列（環境 → ウィンドウ → 描画 → NIF → ループ → ゲームプレイ）
- **16〜25**: 15 完了後、直列または一部並行可能
- **26〜31**: 25 完了後。29/28 を先にするとパフォーマンス効果が大きい
- **32〜40**: 31 完了後。汎用化は 32→33→34→35→36→37→38→39→40 の順が無難
- **41〜47**: 40 完了後。41 を優先すると他ステップの土台ができる。46 は 41 完了後であればいつでも実施可。47 は 42 完了後が望ましい
- **48〜54**: 47 完了後（または並行）。3D・三人称 FPS の実装
- **55〜61**: 54 完了後。Slot・コンポーネントを Elixir で管理する設計への移行とエディタ基盤

---

## 10. 関連ドキュメント

| ドキュメント | 用途 |
|-------------|------|
| [STEPS_BASE.md](./STEPS_BASE.md) | Step 1〜15 の詳細手順・コード |
| [STEPS_QUALITY.md](./STEPS_QUALITY.md) | Step 16〜25 の詳細手順・コード |
| [STEPS_PERF.md](./STEPS_PERF.md) | Step 26〜31 の詳細手順・コード |
| [STEPS_PERFORMANCE_ANALYSIS.md](./STEPS_PERFORMANCE_ANALYSIS.md) | パフォーマンス課題の分析・提案 |
| [STEPS_EXTENSION.md](./STEPS_EXTENSION.md) | Step 40〜47 の詳細 |
| [STEPS_3D.md](./STEPS_3D.md) | Step 48〜54 3D・三人称 FPS の詳細 |
| [STEPS_SLOT_COMPONENT.md](./STEPS_SLOT_COMPONENT.md) | Step 55〜61 Slot・コンポーネントの詳細 |
| [STEPS_GENERALIZATION.md](./STEPS_GENERALIZATION.md) | Step 32〜40 汎用化の詳細 |
| [PRIORITY_STEPS.md](../04_roadmap/PRIORITY_STEPS.md) | 実施優先度（P1〜P7, G1〜G3） |
| [SPEC.md](../01_setup/SPEC.md) | ゲーム仕様・技術仕様 |
