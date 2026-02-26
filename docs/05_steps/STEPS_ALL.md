# 実装ステップ一覧（章・節・項構成）

**目的**: ゲームエンジン実装を**章 → 節 → 項**で整理し、全体像と推奨順序を把握しやすくする。  
**詳細な手順・コード例**は各節の元ドキュメントを参照すること。

**構成**:

- **1. エンジン構築**: 1.1 基礎〜1.12 Slot・コンポーネントまで、ゲームエンジン本体の実装
- **2. エディタ構築**: ビジュアルエディタの実装（詳細は今後決める）
- **3. サーバー構築**: Elixir/Phoenix バックエンド・EOS 等のオンライン化（詳細は今後決める）

**表記例**: **1.1.1** = 1章 1節 1項。各節の表では、文脈が明らかな場合に「○.」と略記する。

---

## 全体ロードマップ


| 章             | 節                 | 項    | 内容                                                                                                                 |
| ------------- | ----------------- | ---- | ------------------------------------------------------------------------------------------------------------------ |
| **1. エンジン構築** | 1. 基礎             | 全15項 | 環境構築〜ゲームオーバー・リスタートまで「動くゲーム」                                                                                        |
|               | 2. クオリティ          | 全10項 | ヒットエフェクト〜ボス・バランスで「楽しめるゲーム」                                                                                         |
|               | 3. パフォーマンス        | 全6項  | イベントバス・ETS・フリーリスト・Spatial Hash・Telemetry・SIMD                                                                      |
|               | 4. 汎用化            | 全9項  | Game インターフェース・シーン汎用化・ゲーム分離・2 つ目のゲーム土台                                                                              |
|               | 5. 拡張             | 全7項  | ゲームループ Rust 移行・マップ・セーブ・マルチ・デバッグ・リネーム・SPEC コンテンツ                                                                    |
|               | 6. Rust lib 分割・整理 | 全9項  | Workspace Layout ツール(xtask)＋3クレート構成＋lib.rs分割。3D・Slot の**前**に実施（[STEPS_RUST_LIB.md](./01_engine/STEPS_RUST_LIB.md)） |
|               | 7. 描画統合（game_window → game_native） | 全8項 | game_window 廃止・game_native へ統合。NIF が描画スレッド spawn、iex -S mix 単一プロセスで wgpu 描画。まずは Windows（[STEPS_RENDER_INTEGRATION.md](./01_engine/STEPS_RENDER_INTEGRATION.md)） |
|               | 8. 描画責務分離（game_native → game_window / game_render） | 全6項 | ウィンドウ管理（winit）と描画コア（wgpu）を分離し、`game_native` を NIF 境界に専念させる（[STEPS_RENDER_SEPARATION.md](./01_engine/STEPS_RENDER_SEPARATION.md)） |
|               | 9. 2Dゲームの固め       | —    | 2D サバイバーを仕様・バランス・品質として固める                                                                                          |
|               | 10. EOS 実装         | —    | 友達・ロビー・セッションを EOS で実装（[EPIC_ONLINE_SERVICES.md](../06_system_design/EPIC_ONLINE_SERVICES.md)）                      |
|               | 11. 3D・三人称FPS      | 全7項  | **据え置き**。WGPU 3D 基盤・カメラ・メッシュ・プレイヤー制御・射撃・敵AI・UI（[STEPS_3D.md](./01_engine/STEPS_3D.md)）                             |
|               | 12. Slot・コンポーネント  | 全7項  | **据え置き**。シーングラフ（Slot）と Component を Elixir で管理（[STEPS_SLOT_COMPONENT.md](./01_engine/STEPS_SLOT_COMPONENT.md)）      |
| **2. エディタ構築** | —                 | —    | ビジュアルエディタの実装。項は今後決める                                                                                               |
| **3. サーバー構築** | —                 | —    | Elixir/Phoenix バックエンド・オンライン化。項は今後決める                                                                               |


---

## 1. エンジン構築

1.1.1〜1.12.7 を、1. 基礎〜12. Slot・コンポーネントの**項**として配置する。

---

### 1.1  基礎（全15項）


| 項   | 目標                       | 備考                |
| --- | ------------------------ | ----------------- |
| 1.  | 環境構築                     |                   |
| 2.  | ウィンドウ表示（Rust クレート・winit） |                   |
| 3.  | wgpu 初期化・単色クリア           |                   |
| 4.  | スプライト 1 枚描画              |                   |
| 5.  | インスタンシング描画（100 体）        |                   |
| 6.  | NIF 連携（Elixir + Rustler） |                   |
| 7.  | ゲームループ（GenServer 60 Hz）  | ※1.5.1 で Rust に移行 |
| 8.  | プレイヤー移動                  |                   |
| 9.  | 敵スポーン・追跡 AI              |                   |
| 10. | 衝突判定（Spatial Hash）       |                   |
| 11. | 武器・弾丸システム                |                   |
| 12. | 大規模スポーン・最適化（5000 体）      |                   |
| 13. | UI（HP・スコア・タイマー）          |                   |
| 14. | レベルアップ・武器選択              |                   |
| 15. | ゲームオーバー・リスタート            |                   |


**詳細**: [STEPS_BASE.md](./01_engine/STEPS_BASE.md)

---

### 1.2  クオリティ（全10項）


| 項   | 目標                                |
| --- | --------------------------------- |
| 1.  | ヒットエフェクト（パーティクル）                  |
| 2.  | 武器レベルアップ（重ね強化）                    |
| 3.  | 複数敵タイプ                            |
| 4.  | アイテムドロップ（回復・磁石）                   |
| 5.  | カメラシステム（プレイヤー追従）                  |
| 6.  | 武器追加（Whip / Fireball / Lightning） |
| 7.  | BGM・SE（rodio）                     |
| 8.  | スプライトアニメーション                      |
| 9.  | ボスエネミー                            |
| 10. | バランス調整・ポリッシュ                      |


**詳細**: [STEPS_QUALITY.md](./01_engine/STEPS_QUALITY.md)

---

### 1.3 パフォーマンス（全6項）


| 項   | 目標                      | 備考                   |
| --- | ----------------------- | -------------------- |
| 1.  | イベントバス                  | フレームイベントを Elixir に配信 |
| 2.  | ETS キャッシュ・入力ポーリング       | プロセス間通信の最適化          |
| 3.  | フリーリスト（スポーン O(1)）       | P3                   |
| 4.  | Spatial Hash 最近接・RwLock | AI 高速化（P1, P2）       |
| 5.  | Telemetry 計測基盤          | 観測可能性（P7）            |
| 6.  | SIMD AI 高速化（オプション）      | P4                   |


**推奨順（パフォーマンス優先）**: 4. → 3. → 1. → 2. → 5. → 6.  
**詳細**: [STEPS_PERF.md](./01_engine/STEPS_PERF.md)  
**分析・課題整理**: [STEPS_PERFORMANCE_ANALYSIS.md](./01_engine/STEPS_PERFORMANCE_ANALYSIS.md)

---

### 1.4 汎用化（全9項）


| 項   | 目標                      |
| --- | ----------------------- |
| 1.  | Game インターフェース設計         |
| 2.  | render_type 汎用化         |
| 3.  | ゲーム切替（config）           |
| 4.  | ゲーム分離（vampire_survivor） |
| 5.  | Game behaviour 実装       |
| 6.  | エンジン API 安定化            |
| 7.  | entity_registry（データ駆動）  |
| 8.  | ゲーム別アセットパス              |
| 9.  | 2 つ目のゲーム（ミニマル実装）        |


**詳細**: [STEPS_GENERALIZATION.md](./01_engine/STEPS_GENERALIZATION.md)

---

### 1.5 拡張（全7項）


| 項   | 目標                                               | 備考                                                |
| --- | ------------------------------------------------ | ------------------------------------------------- |
| 1.  | ゲームループの Rust 移行（高精度 60 Hz）                       | 1.1.7 のゲームループを Rust 側に移行                          |
| 2.  | マップ・障害物システム                                      |                                                   |
| 3.  | セーブ・ロード                                          |                                                   |
| 4.  | マルチプレイ基盤（ルーム管理）                                  |                                                   |
| 5.  | デバッグ支援（NIF）                                      |                                                   |
| 6.  | GameLoop を GameEvents にリネーム                      | Elixir 側の役割（frame_events 受信・フェーズ管理）に合わせてモジュール名を変更 |
| 7.  | SPEC 未実装コンテンツ（Skeleton / Ghost / Garlic / 壁すり抜け） | 2.完了後が望ましい                                        |


**推奨順序**: 1. → 5. → 2. → 3. → 4. → 6.（2.と3.は並行可）。7.は 2.完了後が望ましい。  
**詳細**: [STEPS_EXTENSION.md](./01_engine/STEPS_EXTENSION.md)

---

### 1.6  Rust lib 分割・整理（全9項）


| 項     | 目標                                                                 |
| ----- | ------------------------------------------------------------------ |
| 1.6.1 | Workspace Layout ツール: xtask による WorkspaceLayout.md 自動生成とファイルヘッダー規約 |
| 1.6.2 | Workspace 化: game_core / game_native / game_window の 3 クレート構成に分割   |
| 1.6.3 | ブロック切り出し順序の決定                                                      |
| 1.6.4 | `world/` の作成と型定義の移動                                                |
| 1.6.5 | `game_logic/` の作成とロジックの移動                                          |
| 1.6.6 | `nif/` の作成と NIF 関数の移動                                              |
| 1.6.7 | `lib.rs` のスリム化と動作確認                                                |
| 1.6.8 | Elixir・Mix のビルドパス確認                                                |
| 1.6.9 | ドキュメント更新                                                           |


> 3D・Slot の**前**に実施。1.1〜1.5 完了後に着手する。

**詳細**: [STEPS_RUST_LIB.md](./01_engine/STEPS_RUST_LIB.md)

---

### 1.7  描画統合（game_window → game_native）（全8項）

| 項     | 目標                                                                 |
|--------|----------------------------------------------------------------------|
| 1.7.1  | アーキテクチャとデータ共有方式の確定（描画スレッド spawn・GameWorld 共有方針） |
| 1.7.2  | game_native に renderer を追加（game_window から移動）               |
| 1.7.3  | game_native に asset / audio を追加                                 |
| 1.7.4  | 描画スレッド spawn と winit 統合（NIF で spawn、EventLoop 骨組み）   |
| 1.7.5  | GameWorld から描画データ取得経路の実装（read でスナップショット→renderer） |
| 1.7.6  | Elixir 起動フローとの接続（描画開始 NIF を 1 回呼ぶ）                |
| 1.7.7  | game_window クレートの廃止（workspace・CI・README 更新）              |
| 1.7.8  | Windows 動作確認と FOLDER_CONNECTIONS / ARCHITECTURE 更新            |

1.6 完了後、game_window バイナリを廃止し、renderer / winit / wgpu を game_native に統合。NIF が描画スレッドを spawn し、`iex -S mix` 単一プロセスで wgpu 描画を実行。まずは Windows で動作確認。

**詳細**: [STEPS_RENDER_INTEGRATION.md](./01_engine/STEPS_RENDER_INTEGRATION.md)

---

### 1.8  描画責務分離（game_native → game_window / game_render）（全6項）

| 項     | 目標 |
|--------|------|
| 1.8.1  | 分離方針の確定（責務・依存方向・公開 API） |
| 1.8.2  | `native/game_render` の作成（wgpu パイプライン、描画 API） |
| 1.8.3  | `native/game_window` の再導入（winit 管理、サイズ決定、イベント処理） |
| 1.8.4  | Bridge 実装（`game_native` が frame/input/ui_action を仲介） |
| 1.8.5  | `game_native` から描画実装依存を削減（NIF 境界・GameWorld 連携に集中） |
| 1.8.6  | Windows 動作確認とアーキテクチャ文書更新 |

1.7 で統合した描画を責務分離し、`game_window` はウィンドウ管理、`game_render` は描画コア、`game_native` は NIF/状態管理に専念する構成に整理する。

**詳細**: [STEPS_RENDER_SEPARATION.md](./01_engine/STEPS_RENDER_SEPARATION.md)

---

### 1.9  2Dゲームの固め

1.8 完了後、2D サバイバーを仕様・バランス・品質として固める。項は今後決める。

---

### 1.10 EOS 実装

1.9 完了後、友達・ロビー・セッションを EOS で実装する。項は今後決める。

**詳細**: [EPIC_ONLINE_SERVICES.md](../06_system_design/EPIC_ONLINE_SERVICES.md)

---

### 1.11  3D・三人称 FPS（全7項・据え置き）

> **据え置き**: 本節は当面保留。1.6〜1.10 の後に再検討する。


| 項   | 目標               | 備考                               |
| --- | ---------------- | -------------------------------- |
| 1.  | 3D レンダリング基盤      | wgpu で 3D パイプライン・深度バッファ・頂点/法線/UV |
| 2.  | 3D カメラ・三人称視点     | View/Projection 行列・カメラ追従         |
| 3.  | 3D メッシュ描画・アトラス流用 | 既存スプライトアトラスを 3D テクスチャに           |
| 4.  | 三人称プレイヤー制御       | 移動・カメラ追従・照準（WASD・マウス）            |
| 5.  | 射撃・弾丸・レイキャスト     | FPS 武器・ヒット判定                     |
| 6.  | 敵の 3D スポーン・AI・衝突 | 敵配置・Chase AI・ダメージ・経験値            |
| 7.  | UI・アセット流用・ポリッシュ  | HUD・BGM/SE 流用・ゲーム選択統合            |


**詳細**: [STEPS_3D.md](./01_engine/STEPS_3D.md)

---

### 1.12 Slot・コンポーネント（全7項・据え置き）

> **据え置き**: 本節は当面保留。1.11 完了後に実施する。


| 項   | 目標                              | 備考                                            |
| --- | ------------------------------- | --------------------------------------------- |
| 1.  | Slot（transform 階層）のデータ構造        | Elixir で id / parent_id / local_transform を管理 |
| 2.  | コンポーネント型・レジストリと Slot への付与       | Camera, Player, Enemy, Mesh 等の型と Slot への付与    |
| 3.  | ワールド行列計算・シーンスナップショット・Rust 連携    | 毎フレーム Elixir → Rust にスナップショットを渡して描画           |
| 4.  | 物理結果の Rust → Elixir 反映（Slot 同期） | 物理ステップの結果で Slot の位置を更新                        |
| 5.  | シーンシリアライズ・保存/ロード                | シーンファイル形式・バージョン・ロード                           |
| 6.  | Prefab とインスタンス                  | 再利用可能な Slot サブツリー・インスタンス化とオーバーライド             |
| 7.  | ビジュアルエディタ向け基盤                   | コンポーネントスキーマ公開・選択・Undo の検討と最小実装                |


**詳細**: [STEPS_SLOT_COMPONENT.md](./01_engine/STEPS_SLOT_COMPONENT.md)

---

## 2. エディタ構築

1章（エンジン構築）完了後、ビジュアルエディタの実装に着手する。節・項は今後決める。

---

## 3. サーバー構築

1章完了後、Elixir/Phoenix バックエンド・オンライン化を実装する。節・項は今後決める。  
**参照**: [EPIC_ONLINE_SERVICES.md](../06_system_design/EPIC_ONLINE_SERVICES.md)、[SERVER_DESIGN.md](../06_system_design/SERVER_DESIGN.md)

---

## 依存関係（概要）

- **1.1（全15項）**: 直列（環境 → ウィンドウ → 描画 → NIF → ループ → ゲームプレイ）
- **1.2（全10項）**: 1.1 完了後、直列または一部並行可能
- **1.3（全6項）**: 1.2 完了後。4. / 3. を先にするとパフォーマンス効果が大きい
- **1.4（全9項）**: 1.3 完了後。汎用化は 1.→…→9. の順が無難
- **1.5（全7項）**: 1.4 完了後。1.を優先すると他項の土台ができる。6.は 1.完了後であればいつでも実施可。7.は 2.完了後が望ましい
- **1.6**: 1.5 完了後。3D・Slot の**前**に実施
- **1.7**: 1.6 完了後。描画統合（game_window → game_native）、iex -S mix 単一プロセスで wgpu 描画
- **1.8**: 1.7 完了後。描画責務分離（`game_native` → `game_window` / `game_render`）
- **1.9**: 1.8 完了後。2D の仕様・バランス・品質を固める
- **1.10**: 1.9 完了後。EOS で友達・ロビー・セッションを実装
- **1.11（据え置き）**: 上記が一区切りついた後に再検討
- **1.12（据え置き）**: 1.11 完了後
- **2. エディタ構築**: 1章完了後
- **3. サーバー構築**: 1章完了後

---

## 関連ドキュメント

### 1章 エンジン構築


| ドキュメント                                                                     | 用途                                 |
| -------------------------------------------------------------------------- | ---------------------------------- |
| [STEPS_BASE.md](./01_engine/STEPS_BASE.md)                                 | 1.1（全15項）の詳細手順・コード                 |
| [STEPS_QUALITY.md](./01_engine/STEPS_QUALITY.md)                           | 1.2（全10項）の詳細手順・コード                 |
| [STEPS_PERF.md](./01_engine/STEPS_PERF.md)                                 | 1.3（全6項）の詳細手順・コード                  |
| [STEPS_PERFORMANCE_ANALYSIS.md](./01_engine/STEPS_PERFORMANCE_ANALYSIS.md) | パフォーマンス課題の分析・提案                    |
| [STEPS_EXTENSION.md](./01_engine/STEPS_EXTENSION.md)                       | 1.5（全7項）の詳細                        |
| [STEPS_GENERALIZATION.md](./01_engine/STEPS_GENERALIZATION.md)             | 1.4（全9項）汎用化の詳細                     |
| [STEPS_RUST_LIB.md](./01_engine/STEPS_RUST_LIB.md)                         | 1.6 Rust lib 分割・フォルダ構成検討           |
| [STEPS_RENDER_INTEGRATION.md](./01_engine/STEPS_RENDER_INTEGRATION.md)     | 1.7 描画統合（game_window → game_native）の詳細 |
| [STEPS_RENDER_SEPARATION.md](./01_engine/STEPS_RENDER_SEPARATION.md)       | 1.8 描画責務分離（game_native → game_window / game_render）の詳細 |
| [STEPS_3D.md](./01_engine/STEPS_3D.md)                                     | 1.11（全7項）3D・三人称 FPS の詳細（**据え置き**）   |
| [STEPS_SLOT_COMPONENT.md](./01_engine/STEPS_SLOT_COMPONENT.md)             | 1.12（全7項）Slot・コンポーネントの詳細（**据え置き**） |


### 他フォルダ


| ドキュメント                                                                 | 用途                  |
| ---------------------------------------------------------------------- | ------------------- |
| [EPIC_ONLINE_SERVICES.md](../06_system_design/EPIC_ONLINE_SERVICES.md) | 1.10 EOS 実装          |
| [PRIORITY_STEPS.md](../04_roadmap/PRIORITY_STEPS.md)                   | 実施優先度（P1〜P7, G1〜G3） |
| [SPEC.md](../01_setup/SPEC.md)                                         | ゲーム仕様・技術仕様          |


