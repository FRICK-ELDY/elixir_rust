# ゲームエンジン分析: 再評価版（コードベース準拠）

**対象プロジェクト**: Elixir × Rust Survivor  
**アーキテクチャ**: Elixir (OTP ゲームロジック) + Rust (物理演算・GPU レンダリング) ハイブリッド  
**作成日**: 2026-02-23  
**評価の根拠**: **コードベースを直接読み、実装済みの現状に基づく評価**（ロードマップではない）

> **元の分析**: [ENGINE_ANALYSIS.md](./ENGINE_ANALYSIS.md)（アーカイブ）

---

## 目次

1. [現状の実装（コード確認済み）](#1-現状の実装コード確認済み)
2. [強み・メリット（現状）](#2-強みメリット現状)
3. [残存する弱み・課題](#3-残存する弱み課題)
4. [総合評価](#4-総合評価)

---

## 1. 現状の実装（コード確認済み）

コードベースを確認した結果、以下の項目は**すべて実装済み**です。

### 1.1 パフォーマンス（Rust コア）

| 項目 | 実装箇所 | 状態 |
|------|----------|------|
| **Spatial Hash 最近接探索** | `lib.rs` L438: `find_nearest_enemy_spatial`, L460: `find_nearest_enemy_spatial_excluding` | ✅ |
| **Lightning チェーン空間ハッシュ化** | `lib.rs` L1067, L1111: `find_nearest_enemy_spatial_excluding` で連鎖先探索 | ✅ |
| **RwLock** | `lib.rs` L23, L735: `GameWorld(pub RwLock<GameWorldInner>)` | ✅ |
| **フリーリスト** | BulletWorld, ParticleWorld, EnemyWorld, ItemWorld 各 SoA に `free_list: Vec<usize>` | ✅ |
| **SIMD Chase AI** | `lib.rs` L507: `update_chase_ai_simd` (x86_64)、L834 で `physics_step` から呼び出し | ✅ |

### 1.2 パフォーマンス（Elixir レイヤー）

| 項目 | 実装箇所 | 状態 |
|------|----------|------|
| **イベントバス** | `lib/game/event_bus.ex`, `application.ex` 登録、`game_loop.ex` で `drain_frame_events` → broadcast | ✅ |
| **ETS FrameCache** | `lib/game/frame_cache.ex`, `game_loop.ex` init/put, `stress_monitor.ex` get | ✅ |
| **ETS 入力ポーリング** | `lib/game/input_handler.ex`: `get_move_vector/0` で ETS 読み取り、`game_loop.ex` L122 で毎 tick 呼び出し | ✅ |
| **Telemetry** | `game_loop.ex` L192, L234, `scenes/playing.ex` L36, L51, `mix.exs` 依存 | ✅ |

### 1.3 汎用化基盤

| 項目 | 実装箇所 | 状態 |
|------|----------|------|
| **main/lib 共通ロジック統合** | `native/game_native/src/core/` に enemy, boss, item, physics, weapon, util を集約。`main.rs` と `lib.rs` 両方で `mod core` | ✅ |
| **シーン管理** | `lib/game/scene_manager.ex`, `scene_behaviour.ex`, `scenes/playing.ex`, `level_up.ex`, `boss_alert.ex`, `game_over.ex` | ✅ |
| **アセット管理** | `native/game_native/src/asset/mod.rs`: `AssetLoader`, 実行時ロード + `include_bytes!` フォールバック | ✅ |

### 1.4 品質・拡張

| 項目 | 実装箇所 | 状態 |
|------|----------|------|
| **NIF オーバーヘッド対策** | `get_frame_metadata` NIF で HUD 等を 1 回取得。`get_render_data` は deprecated、描画は Rust 内完結 | ✅ |
| **テストコード** | Elixir: `test/spawn_system_test.exs`, `level_system_test.exs`, `boss_system_test.exs`。Rust: `core/util.rs`, `weapon.rs`, `enemy.rs` 等に `#[test]` | ✅ |

---

## 2. 強み・メリット（現状）

### 2.1 パフォーマンス面

- **GPU インスタンシング描画** — wgpu による 1 draw call 大量描画
- **SoA ECS** — EnemyWorld, BulletWorld, ParticleWorld, ItemWorld が SoA + フリーリストで O(1) スポーン
- **空間ハッシュ** — 衝突判定と最近接探索の両方に活用、O(n) 全探索を回避
- **rayon 並列 AI** — Chase AI をデータ並列化。x86_64 では SIMD 版でさらに高速化
- **RwLock** — 読み取り専用 NIF が並行実行可能、StressMonitor と GameLoop の競合解消

### 2.2 信頼性・耐障害性面

- **OTP Supervisor** — SceneManager, InputHandler, EventBus, GameLoop, StressMonitor, Stats, Telemetry を `one_for_one` で監視
- **EventBus** — フレームイベントを Stats 等にノンブロッキング配信、ゲームループへの影響なし
- **ETS** — FrameCache と InputState でプロセス間ロックフリー共有

### 2.3 開発体験・保守性面

- **シーン管理** — Playing, LevelUp, BossAlert, GameOver を独立シーンとして分離、`SceneBehaviour` で init/update を定義
- **core モジュール** — main.rs と lib.rs の重複を解消、定数・武器・敵・物理を 1 箇所に集約
- **テスト** — SpawnSystem, LevelSystem, BossSystem の純粋関数と Rust 側ユーティリティをカバー

### 2.4 観測可能性

- **Telemetry** — `[:game, :tick]`, `[:game, :level_up]`, `[:game, :boss_spawn]` 等を発火、LiveDashboard / Prometheus 連携の土台

---

## 3. 残存する弱み・課題

コード確認時点で**未対応または部分的**な項目です。

### 3.1 パフォーマンス面

| 課題 | 現状 |
|------|------|
| **60Hz ゲームループのジッター** | `Process.send_after/3` は Erlang スケジューラ依存で ±数 ms のジッターが発生。高精度タイマーが必要な場合は別検討 |

### 3.2 機能面

| 課題 | 現状 |
|------|------|
| **完全な ECS フレームワークではない** | SoA を手動実装。エンティティ間の親子・グループ関係やコンポーネントの動的追加はない |
| **マップ・タイル管理がない** | 無限平面のみ。障害物・壁・タイルマップの概念なし |
| **セーブ・ロード機能がない** | ゲーム状態の永続化なし |
| **未実装仕様** | Skeleton, Ghost, Garlic 等は仕様書にあれど未実装 |
| **UI/UX の制限** | egui 即時モード、リッチテキスト・多言語対応は限定的 |

### 3.3 スケーラビリティ面

| 課題 | 現状 |
|------|------|
| **シングルプレイヤー前提** | マルチプレイヤー対応には GameWorld の設計変更が必要 |
| **ネットワーク機能なし** | オンラインマルチ・ランキング・実績連携は未実装。Elixir のネットワーク機能は未活用 |

### 3.4 開発体験面

| 課題 | 現状 |
|------|------|
| **デバッグの困難さ** | NIF クラッシュは BEAM VM ごとクラッシュ。Rust 側パニックが Elixir スタックトレースに表示されない |
| **ビルド複雑性** | Elixir + Rust のデュアルビルド、音声ファイルは事前生成が必要 |

---

## 4. 総合評価

### 4.1 現状の位置づけ

ロードマップ（PRIORITY_STEPS, STEPS_PERF）で挙がっていた改善項目は**ほぼすべてコードに反映済み**です。  
パフォーマンス・汎用化・品質の基盤が整い、「技術デモ」の域を超えて**実用に近いゲームエンジン**になっています。

### 4.2 技術的独自性

- **OTP による耐障害性** — ゲームエンジンとしては珍しい、プロダクションレベルの障害回復
- **分散システムへの拡張性** — Elixir/OTP の設計思想により、マルチノード展開が他エンジンより容易
- **Elixir と Rust の役割分担** — ゲームロジックは宣言的・関数型、物理・描画は高性能・型安全

### 4.3 一言まとめ

> **「本番サービスの信頼性でゲームを動かす」というアプローチを、コードレベルで実現している。**  
> **パフォーマンス最適化（空間ハッシュ、フリーリスト、RwLock、SIMD）、Elixir レイヤーの OTP 活用（EventBus、ETS、Telemetry）、汎用化基盤（シーン管理、アセットローダー、core 統合）が揃っている。**  
> **残る課題は主に機能拡張（マップ、セーブ、マルチプレイ）とデバッグ支援であり、アーキテクチャ上の弱点ではない。**

### 4.4 関連ドキュメント

| ドキュメント | 用途 |
|-------------|------|
| [ENGINE_ANALYSIS.md](./ENGINE_ANALYSIS.md) | 元の分析（アーカイブ） |
| [ELIXIR_RUST_DIVISION.md](../03_tech_decisions/ELIXIR_RUST_DIVISION.md) | Elixir/Rust 役割分担方針、やらなくていいもの |
| [PRIORITY_STEPS.md](../04_roadmap/PRIORITY_STEPS.md) | 実装済み項目の参照 |
| [STEPS_PERF.md](../05_steps/STEPS_PERF.md) | 実装の詳細手順 |
| [ASSET_MANAGEMENT.md](../06_system_design/ASSET_MANAGEMENT.md) | アセット管理設計 |
