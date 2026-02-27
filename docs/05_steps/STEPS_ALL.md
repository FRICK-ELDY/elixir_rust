# 実装ステップ一覧（章・節・項構成）

**目的**: ゲームエンジン実装を**章 → 節 → 項**で整理し、全体像と推奨順序を把握しやすくする。  
**詳細な手順・コード例**は各節の元ドキュメントを参照すること。

**構成**:

- **1. エンジン構築**: 1.1 基礎〜1.13 Slot・コンポーネントまで、ゲームエンジン本体の実装
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
|               | 9. アーキテクチャ改善     | 全5項 | Rust/Elixir 境界と責務を整理し、`physics_step`・`renderer/mod.rs`・NIF 契約を段階的に改善（[ARCHITECTURE_IMPROVEMENT.md](../06_system_design/ARCHITECTURE_IMPROVEMENT.md)） |
|               | 10. 方針決定とアーキテクチャ再構築 | 全10項 | Elixir SSOT・Push 型同期・tick_hz 可変・Umbrella 化・game_network 分離（[STEPS_ARCH_REDESIGN.md](./01_engine/STEPS_ARCH_REDESIGN.md)） |
|               | 11. EOS 実装         | —    | 友達・ロビー・セッションを EOS で実装（[EPIC_ONLINE_SERVICES.md](../06_system_design/EPIC_ONLINE_SERVICES.md)）                      |
|               | 12. 3D・三人称FPS      | 全7項  | **据え置き**。WGPU 3D 基盤・カメラ・メッシュ・プレイヤー制御・射撃・敵AI・UI（[STEPS_3D.md](./01_engine/STEPS_3D.md)）                             |
|               | 13. Slot・コンポーネント  | 全7項  | **据え置き**。シーングラフ（Slot）と Component を Elixir で管理（[STEPS_SLOT_COMPONENT.md](./01_engine/STEPS_SLOT_COMPONENT.md)）      |
| **2. エディタ構築** | —                 | —    | ビジュアルエディタの実装。項は今後決める                                                                                               |
| **3. サーバー構築** | —                 | —    | Elixir/Phoenix バックエンド・オンライン化。項は今後決める                                                                               |


---

## 1. エンジン構築

1.1.1〜1.13.7 を、1. 基礎〜13. Slot・コンポーネントの**項**として配置する。

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

### 1.9  アーキテクチャ改善（全5項）

| 項     | 目標 |
|--------|------|
| 1.9.1  | `game_native/systems` 配下の再編方針を確定（`physics_step.rs` 分割単位・依存方向・移行順） |
| 1.9.2  | `physics_step.rs` を機能別モジュールへ分割（移動・衝突・ダメージ・ドロップ等） |
| 1.9.3  | `game_render/renderer/mod.rs` を facade 化し、pipeline/buffers/sprites/ui へ分割 |
| 1.9.4  | Elixir 側を command/query 入口に整理し、`App.NifBridge` 呼び出しを集約 |
| 1.9.5  | 旧 API を deprecate して段階置換、最終削除。Windows 動作確認と設計文書更新 |

1.8 完了後、責務集中したモジュールを分割し、Rust/Elixir の境界契約を明確化する。最終的に変更影響範囲を小さくし、差し替え性と保守性を高める。

**詳細**: [STEPS_ARCHITECTURE_IMPROVEMENT.md](./01_engine/STEPS_ARCHITECTURE_IMPROVEMENT.md)  
**参照設計**: [ARCHITECTURE_IMPROVEMENT.md](../06_system_design/ARCHITECTURE_IMPROVEMENT.md)

---

### 1.10  方針決定とアーキテクチャ再構築（全10項）

| 項      | 目標                                                                 | 備考 |
|---------|----------------------------------------------------------------------|------|
| 1.10.1  | 設計方針の確定と ADR 更新（Elixir SSOT・Push 型同期・tick_hz 可変）  | ADR_SHARED_MEMORY_THREAD_POLICY.md / ELIXIR_RUST_DIVISION.md 更新済み |
| 1.10.2  | Umbrella プロジェクト化（ルート mix.exs 作成・apps/ 配置）           | 現 `:game` アプリを `apps/game_engine` へ移動 |
| 1.10.3  | `game_engine` アプリ整備（NIF ロード・tick_hz 設定・ヘッドレス対応） | `Application.get_env(:game_engine, :tick_hz, 20)` で可変化 |
| 1.10.4  | `game_content` アプリ分離（`lib/games` → `apps/game_content`）      | `game_engine` への依存のみ。ゲーム別ロジックを隔離 |
| 1.10.5  | Push 型同期 NIF の実装（`push_snapshot` / `physics_result`）         | 旧 `physics_step` NIF を Push 型に置き換え |
| 1.10.6  | Rust 計算スレッド・描画スレッド・音スレッドの 60Hz 独立化            | 各スレッドを `tick_hz` に依存しない構成に整理 |
| 1.10.7  | 描画スレッドの補間実装（スナップショット → 60Hz 補間描画）            | スナップショット間を線形補間してフレームを生成 |
| 1.10.8  | `game_network` アプリ新規作成（Phoenix Socket / Channel・認証・Presence） | ネットワーク汎用層を `game_engine` から分離 |
| 1.10.9  | `game_server` アプリ新規作成（本番デプロイ用エントリ・設定集約）      | サーバー起動時は Rust スレッドをロードしない（ヘッドレス） |
| 1.10.10 | 動作確認・設計文書更新（ARCHITECTURE.md / FOLDER_CONNECTIONS.md）    | ローカル起動・サーバー起動の両方で動作確認 |

1.9 完了後、エンジンの設計方針を Elixir SSOT + Push 型同期に切り替え、Umbrella 構成でスケーラブルな Elixir 側を構築する。ネットワーク層（`game_network`）を汎用アプリとして独立させ、ローカルクライアントとサーバーデプロイを同一コードベースで切り替えられる構成を目指す。

**詳細**: [STEPS_ARCH_REDESIGN.md](./01_engine/STEPS_ARCH_REDESIGN.md)  
**参照 ADR**: [ADR_SHARED_MEMORY_THREAD_POLICY.md](../03_tech_decisions/ADR_SHARED_MEMORY_THREAD_POLICY.md)  
**参照設計**: [ELIXIR_RUST_DIVISION.md](../03_tech_decisions/ELIXIR_RUST_DIVISION.md)

---

### 1.11 EOS 実装

1.10 完了後、友達・ロビー・セッションを EOS で実装する。項は今後決める。

**詳細**: [EPIC_ONLINE_SERVICES.md](../06_system_design/EPIC_ONLINE_SERVICES.md)

---

### 1.12  3D・三人称 FPS（全7項・据え置き）

> **据え置き**: 本節は当面保留。1.6〜1.11 の後に再検討する。


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

### 1.13 Slot・コンポーネント（全7項・据え置き）

> **据え置き**: 本節は当面保留。1.12 完了後に実施する。


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
- **1.9**: 1.8 完了後。アーキテクチャ改善（`physics_step` 分割・`renderer` 分割・NIF 契約整理）
- **1.10**: 1.9 完了後。方針決定とアーキテクチャ再構築（Elixir SSOT・Push 型・Umbrella 化・game_network 分離）
- **1.11**: 1.10 完了後。EOS で友達・ロビー・セッションを実装
- **1.12（据え置き）**: 上記が一区切りついた後に再検討
- **1.13（据え置き）**: 1.12 完了後
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
| [STEPS_ARCHITECTURE_IMPROVEMENT.md](./01_engine/STEPS_ARCHITECTURE_IMPROVEMENT.md) | 1.9 アーキテクチャ改善（実施ステップ）の詳細 |
| [STEPS_ARCH_REDESIGN.md](./01_engine/STEPS_ARCH_REDESIGN.md)               | 1.10 方針決定とアーキテクチャ再構築（全10項）の詳細 |
| [STEPS_3D.md](./01_engine/STEPS_3D.md)                                     | 1.12（全7項）3D・三人称 FPS の詳細（**据え置き**）   |
| [STEPS_SLOT_COMPONENT.md](./01_engine/STEPS_SLOT_COMPONENT.md)             | 1.13（全7項）Slot・コンポーネントの詳細（**据え置き**） |


### 他フォルダ


| ドキュメント                                                                 | 用途                  |
| ---------------------------------------------------------------------- | ------------------- |
| [EPIC_ONLINE_SERVICES.md](../06_system_design/EPIC_ONLINE_SERVICES.md) | 1.11 EOS 実装          |
| [ARCHITECTURE_IMPROVEMENT.md](../06_system_design/ARCHITECTURE_IMPROVEMENT.md) | 1.9 改善アーキテクチャ設計 |
| [ADR_SHARED_MEMORY_THREAD_POLICY.md](../03_tech_decisions/ADR_SHARED_MEMORY_THREAD_POLICY.md) | 1.10 設計方針 ADR |
| [ELIXIR_RUST_DIVISION.md](../03_tech_decisions/ELIXIR_RUST_DIVISION.md) | 1.10 役割分担方針 |
| [PRIORITY_STEPS.md](../04_roadmap/PRIORITY_STEPS.md)                   | 実施優先度（P1〜P7, G1〜G3） |
| [SPEC.md](../01_setup/SPEC.md)                                         | ゲーム仕様・技術仕様          |
