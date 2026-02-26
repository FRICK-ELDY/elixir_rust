# 1.9 アーキテクチャ改善

**所属**: [STEPS_ALL.md](../STEPS_ALL.md) 1章 エンジン構築 の 1.9 節。  
**参照元設計**: [ARCHITECTURE_IMPROVEMENT.md](../../06_system_design/ARCHITECTURE_IMPROVEMENT.md)

**目的**: 責務集中したモジュールを分割し、Rust/Elixir の境界契約を明確化する。

- `native/game_native/src/game_logic/physics_step.rs` の責務分散
- `native/game_render/src/renderer/mod.rs` の facade 化と内部モジュール分割
- `App.NifBridge` の command/query 境界整理と API 面積の縮小

**前提**: 1.8 描画責務分離（`game_native` / `game_window` / `game_render`）が完了していること。

---

## 概要

| 項目 | 内容 |
|------|------|
| **狙い** | 変更影響範囲の局所化、保守性向上、差し替え容易化 |
| **主対象** | `lib/app`, `lib/engine`, `lib/games`, `native/game_core`, `native/game_native`, `native/game_render`, `native/game_window` |
| **境界方針** | Elixir は command/query を明示、Rust は application/domain/render を分離 |
| **実施順序** | 1.9.1 → 1.9.2 → 1.9.3 → 1.9.4 → 1.9.5 |
| **対象プラットフォーム** | まずは Windows で検証 |

---

## 1.9 節 全体ロードマップ（1.9.1〜1.9.5）

| 項 | 目標 |
|----|------|
| **1.9.1** | `game_native/systems` 再編方針の確定（分割単位・依存方向・移行順） |
| **1.9.2** | `physics_step.rs` を機能別に分割（移動・衝突・ダメージ・ドロップ等） |
| **1.9.3** | `game_render/renderer/mod.rs` の facade 化と描画モジュール分割 |
| **1.9.4** | Elixir 側の command/query 入口整理と `App.NifBridge` 呼び出し集約 |
| **1.9.5** | 旧 API deprecate → 段階置換 → 削除、Windows 動作確認、設計文書更新 |

### 進捗メモ（2026-02-26）

- [x] 1.9.2 の初期分割として `game_logic/systems` を導入し、`spawn` / `leveling` を `physics_step.rs` から移設。
- [x] 1.9.2 の追加分割として `collision` / `effects` / `items` / `projectiles` / `boss` を `systems` に分離し、`physics_step.rs` をオーケストレーション中心へ整理。
- [x] 1.9.3 の初期分割として `renderer/ui.rs` を追加し、`renderer/mod.rs` の HUD/UI 実装を分離。
- [x] 1.9.4 として `Engine.Commands` / `Engine.Queries` を追加し、`App.NifBridge` 直接呼び出しを集約。
- [x] 1.9.5 として旧描画取得 API（`get_render_data` / `get_particle_data` / `get_item_data`）を削除。
- [x] 1.9.5 として Windows で `iex.bat -S mix` の起動確認を実施（IEx プロンプト到達を確認）。
- [ ] 1.9.2 の残タスク（武器発射処理ブロックの更なる機能別分離）
- [ ] 1.9.5 の残タスク（不要化した旧APIに関する関連ドキュメントの全面更新）

---

## 1.9.1 `systems` 再編方針の確定

### 実施内容

- `physics_step.rs` の責務を機能軸で棚卸しし、分割単位を定義する
- 依存方向を明示する（`game_logic -> world` を維持し、逆依存を作らない）
- 移行順序を決める（小さく分割して都度ビルド可能な順序）

### 期待成果物

- 分割マップ（旧関数 → 新モジュール）
- `mod.rs` 更新方針
- 影響範囲一覧（呼び出し元・テスト対象）

### 完了条件

- 分割単位と依存方向がレビュー合意済み
- 1.9.2 の実装に着手可能な粒度でタスク化されている

---

## 1.9.2 `physics_step.rs` 分割

### 実施内容

- `physics_step.rs` の処理を機能別モジュールへ移設する
- 例: 移動、衝突、ダメージ、ドロップ、イベント蓄積など
- 既存公開 API の互換を保ちながら内部実装を段階的に置換する

### 実施時の注意

- 1 回の変更量を小さく保ち、各段階で `cargo check` を通す
- ロジック移動時に副作用順序（イベント発行順・計算順）を維持する

### 完了条件

- `physics_step.rs` の責務集中が解消されている
- 既存ゲーム進行（移動・衝突・ダメージ・ドロップ）が回帰していない

---

## 1.9.3 `renderer/mod.rs` facade 化

### 実施内容

- `native/game_render/src/renderer/mod.rs` を入口（facade）に限定する
- 実装を `pipeline` / `buffers` / `sprites` / `ui` などへ分割する
- facade は初期化・フレーム描画・リサイズ等の公開 API のみ提供する

### 実施時の注意

- `game_window` との接続面は最小変更で維持する
- エラー変換とログ方針を facade 境界に寄せる

### 完了条件

- `renderer/mod.rs` がオーケストレーション中心の薄い層になっている
- 内部実装の差し替えで外部呼び出し側変更が不要になっている

---

## 1.9.4 Elixir 境界整理（command/query）

### 実施内容

- `engine` 側に command/query の入口を定義し、呼び出し責務を分離する
- `App.NifBridge` への直接呼び出しを集約し、境界契約を明確化する
- 既存呼び出し箇所を新入口へ段階移行する

### 実施時の注意

- 既存ゲームモジュールの差し替え性（adapter 構造）を壊さない
- 呼び出し名・引数・戻り値の契約変更は deprecate 期間を設ける

### 完了条件

- command/query の入口がコード上で明確
- NIF 呼び出し経路が整理され、直接依存が減っている

---

## 1.9.5 旧 API 置換・削除と最終確認

### 実施内容

- 旧入口に deprecate 注記を追加し、利用箇所を新入口へ移行する
- 移行完了後に旧 API を削除する
- Windows で `iex -S mix` 動作確認を実施する
- 設計文書を最新構成に更新する

### 更新対象ドキュメント（目安）

- `docs/06_system_design/ARCHITECTURE.md`
- `docs/06_system_design/FOLDER_CONNECTIONS.md`
- `docs/06_system_design/ARCHITECTURE_IMPROVEMENT.md`（実施状況の反映）

### 完了条件

- 旧 API が削除され、新入口で機能が成立する
- Windows で起動・描画・入力・ゲーム進行に問題がない
- 設計文書と実装構成が一致している

---

## リスクと対策

- **副作用順序の破壊**: 1.9.2 は小分け移行し、段階ごとに動作確認する
- **境界変更の波及**: 1.9.4 は互換レイヤーを短期維持して移行する
- **責務再肥大化**: facade には実装を持ち込まず、内部モジュールに閉じ込める

---

## 関連ドキュメント

| ドキュメント | 用途 |
|-------------|------|
| [STEPS_RENDER_SEPARATION.md](./STEPS_RENDER_SEPARATION.md) | 1.8 完了状態（分離済み前提）の確認 |
| [ARCHITECTURE_IMPROVEMENT.md](../../06_system_design/ARCHITECTURE_IMPROVEMENT.md) | 改善アーキテクチャの設計方針 |
| [ARCHITECTURE.md](../../06_system_design/ARCHITECTURE.md) | 全体構成の参照 |
| [FOLDER_CONNECTIONS.md](../../06_system_design/FOLDER_CONNECTIONS.md) | 依存方向・接続関係の参照 |
