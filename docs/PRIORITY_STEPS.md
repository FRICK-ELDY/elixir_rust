# 実装優先度ステップガイド

**根拠**: [ENGINE_ANALYSIS.md](./ENGINE_ANALYSIS.md) の分析に基づく  
**方針**: **パフォーマンス最優先** → **汎用化重視** → 品質・拡張

本ドキュメントは、改善項目を「何から手をつけるか」の優先度で整理したロードマップです。  
詳細な実装手順は [STEPS_PERF.md](./STEPS_PERF.md) を参照してください。

---

## 全体マップ

```
フェーズ1: パフォーマンス — Rust コア（即効性が高い）
  P1  find_nearest_enemy + Lightning チェーン の空間ハッシュ化
  P2  RwLock への変更（読み取り競合解消）
  P3  Rust フリーリスト（スポーン O(1) 化）
  P4  SIMD AI 高速化（オプション・上級）

フェーズ2: パフォーマンス — Elixir レイヤー
  P5  イベントバス（OTP の関心分離）
  P6  ETS キャッシュ + 入力ポーリング化
  P7  Telemetry 計測基盤

フェーズ3: 汎用化基盤
  G1  main.rs と lib.rs の共通ロジック統合
  G2  シーン管理システムの導入
  G3  アセット管理システムの設計

フェーズ4: 品質・拡張
  Q1  基本的なテストコードの追加
  Q2  NIF オーバーヘッド対策（将来検討）
```

---

## 優先度の理由

### パフォーマンスを最優先にする理由

- **即効性**: 敵数 1,000 体以上で `find_nearest_enemy` がボトルネックになる（O(n) 線形探索）
- **計測可能**: `StressMonitor` の `physics_ms` で効果を数値確認できる
- **影響範囲が明確**: Rust 側のみの変更で、Elixir コードへの波及が少ない

### 汎用化を第2に置く理由

- **main.rs / lib.rs 重複**: 1628 行と 1683 行の同期コストが開発負荷になっている
- **シーン管理**: ゲームフェーズのハードコードを解消し、ステージ選択・設定画面などを追加可能に
- **アセット管理**: 実行時ロードができれば大規模ゲームへスケール可能

---

## フェーズ1: パフォーマンス — Rust コア

### P1: find_nearest_enemy + Lightning チェーン の空間ハッシュ化

**根拠**: ENGINE_ANALYSIS「高優先度」・敵数 1000 体以上でボトルネック

| 項目 | 内容 |
|------|------|
| **問題** | Magic Wand / Fireball / Lightning が毎フレーム O(n) 全敵探索 |
| **解決** | 既存 Spatial Hash を活用し、`search_radius` 内の候補のみ探索 |
| **Lightning** | 連鎖先の探索も同様に空間ハッシュで候補を絞る（最大 6 チェーン × O(n) → O(数十)） |
| **参照** | STEPS_PERF Step 29.1, 29.2 |

**詳細手順**: [STEPS_PERF.md § Step 29](./STEPS_PERF.md#step-29-spatial-hash-最近接-rwlock)（29.1, 29.2）

**追加タスク**（Lightning チェーン）:
- `physics_step` 内の Lightning 武器処理で、`find_nearest_enemy_spatial` を連鎖ごとに呼ぶ
- 既にヒットした敵を除外する `exclude` リストを渡すバリアントを追加するか、ヒット済みフラグでフィルタ

---

### P2: RwLock への変更

**根拠**: Mutex によるシリアル化で読み取り専用 NIF がブロックされる

| 項目 | 内容 |
|------|------|
| **問題** | `GameWorld(Mutex<...)` により、`get_hud_data` 等の読み取りも排他される |
| **解決** | `RwLock` に変更。書き込み NIF は `.write()`、読み取り NIF は `.read()` |
| **効果** | StressMonitor と GameLoop が同時に NIF を呼んでもデッドロックしない |
| **参照** | STEPS_PERF Step 29.3 |

**詳細手順**: [STEPS_PERF.md § Step 29](./STEPS_PERF.md#step-29-spatial-hash-最近接-rwlock)（29.3）

---

### P3: Rust フリーリスト

**根拠**: スポーン時の O(n) 線形スキャンが大量エンティティで負荷になる

| 項目 | 内容 |
|------|------|
| **問題** | `BulletWorld` / `ParticleWorld` / `EnemyWorld` の空きスロット探索が O(n) |
| **解決** | `free_list: Vec<usize>` で空きインデックスをスタック管理、O(1) で取得・返却 |
| **参照** | STEPS_PERF Step 28 |

**詳細手順**: [STEPS_PERF.md § Step 28](./STEPS_PERF.md#step-28-rust-フリーリストスポーン-o1-化)

---

### P4: SIMD AI 高速化（オプション）✅ 実装済み

**根拠**: さらなる AI 計算の高速化。x86_64 限定・ARM はフォールバック必須

| 項目 | 内容 |
|------|------|
| **内容** | `update_chase_ai` を SSE2 SIMD で 4 要素同時処理 |
| **前提** | P1〜P3 を完了してから実施。ベンチマークで効果を確認 |
| **参照** | STEPS_PERF Step 31 |

**実装内容**:
- `update_chase_ai_simd`: x86_64 向け SSE2 実装（`_mm_rsqrt_ps` で逆平方根を高速化）
- `physics_step` で `#[cfg(target_arch = "x86_64")]` により自動切り替え
- ベンチマーク: `cargo bench --bench ai_bench` で比較可能

**詳細手順**: [STEPS_PERF.md § Step 31](./STEPS_PERF.md#step-31-simd-ai-高速化上級オプション)

---

## フェーズ2: パフォーマンス — Elixir レイヤー

### P5: イベントバス

**根拠**: OTP の関心分離。Stats が未使用のままになっている問題の解消

| 項目 | 内容 |
|------|------|
| **問題** | `physics_step` の戻り値（イベント）が捨てられ、`Stats` が機能していない |
| **解決** | `FrameEvent` バッファ + `drain_frame_events` NIF + `EventBus` GenServer |
| **効果** | 以降のステップで「ゲームループを触らずに」リプレイ・実績などを追加可能 |
| **参照** | STEPS_PERF Step 26 |

**詳細手順**: [STEPS_PERF.md § Step 26](./STEPS_PERF.md#step-26-イベントバスotp-の関心分離)

---

### P6: ETS キャッシュ + 入力ポーリング化

**根拠**: プロセス間通信のボトルネック解消

| 項目 | 内容 |
|------|------|
| **問題** | StressMonitor の `call` で GameLoop ブロック、InputHandler の `cast` でメッセージキュー圧迫 |
| **解決** | ETS にスナップショット・入力状態を書き込み、ロックフリー読み取り |
| **参照** | STEPS_PERF Step 27 |

**詳細手順**: [STEPS_PERF.md § Step 27](./STEPS_PERF.md#step-27-ets-キャッシュ-入力ポーリング化)

---

### P7: Telemetry 計測基盤

**根拠**: 改善効果を標準的な方法で計測。LiveDashboard / Prometheus 連携の基盤

| 項目 | 内容 |
|------|------|
| **内容** | `:telemetry` で `[:game, :tick]` 等のイベントを発火 |
| **効果** | 計測コードとゲームロジックの分離、エコシステム連携 |
| **参照** | STEPS_PERF Step 30 |

**詳細手順**: [STEPS_PERF.md § Step 30](./STEPS_PERF.md#step-30-telemetry-計測基盤)

---

## フェーズ3: 汎用化基盤

### G1: main.rs と lib.rs の共通ロジック統合 ✅ 実装済み

**根拠**: ENGINE_ANALYSIS「高優先度」・重複管理コストの解消

| 項目 | 内容 |
|------|------|
| **問題** | スタンドアロン (`main.rs`) と NIF (`lib.rs`) でゲームロジックが重複 |
| **解決** | `game_core` 等の共通クレートに `GameWorld` / 物理 / 武器を集約 |
| **成果物** | `main.rs` は winit ループ + `core` 呼び出しのみ、`lib.rs` は NIF ラッパーのみ |

**実装内容**:
- `native/game_native/src/core/` に共通モジュールを集約
- `constants`, `item`, `weapon`, `physics` を core 配下に移動
- `enemy`, `boss`, `util` を追加（EnemyKind, BossKind, exp_required_for_next, spawn_position_outside 等）
- `main.rs` と `lib.rs` の両方から `mod core` を参照

---

### G2: シーン管理システムの導入 ✅ 実装済み

**根拠**: ゲームフェーズのハードコード解消、複数ゲームモード対応

| 項目 | 内容 |
|------|------|
| **現状** | `GamePhase` enum で Title / Playing / LevelUp / BossAlert / GameOver を直列管理 |
| **目標** | シーンスタック（push/pop）、各シーンが独立した init/update/draw を持つ |
| **効果** | ステージ選択・設定画面・チュートリアル等を追加しやすくなる |

**実装内容**:
- `Game.SceneBehaviour` でシーンコールバック（init/update/render_type）を定義
- `Game.SceneManager` GenServer でシーンスタック（push/pop/replace）を管理し render_type を取得
- シーンを `%{module: Module, state: term}` で表現
- `Game.Scenes.Playing`, `LevelUp`, `BossAlert`, `GameOver` で各フェーズを独立シーンに分離
- `GameLoop` を SceneManager ベースにリファクタし、tick を現在シーンの update にディスパッチ
- `FrameCache` に render_type を追加（描画用シーン種別の参照）

---

### G3: アセット管理システムの設計

**根拠**: 実行時ロードがないため大規模ゲームにスケールしない

| 項目 | 内容 |
|------|------|
| **現状** | スプライトアトラスを `include_bytes!` でバイナリ埋め込み |
| **目標** | アセット ID → パス のマッピング、非同期ロード、キャッシュ |
| **段階** | まず設計ドキュメントを作成し、Phase 1 のスプライト差し替えから開始 |

---

## フェーズ4: 品質・拡張

### Q1: 基本的なテストコードの追加

**根拠**: ENGINE_ANALYSIS「高優先度」・現状 `test/` が存在しない

| 項目 | 内容 |
|------|------|
| **優先** | Elixir 側: `SpawnSystem`, `LevelSystem`, `BossSystem` の純粋関数 |
| **次点** | Rust 側: `cargo test` で `physics` / `weapon` の単体テスト |
| **NIF** | 統合テストは IEx から `NifBridge` を呼ぶ E2E で代替可能 |

---

### Q2: NIF オーバーヘッド対策（将来検討）

**根拠**: `get_render_data()` が毎フレーム大量データを Elixir に返す設計

| 項目 | 内容 |
|------|------|
| **現状** | 描画データが Elixir 経由で受け渡されており、ゼロコピーではない |
| **選択肢** | Rust 側で描画ループを完結させる、または NIF でバイナリを返し Elixir では透過的に扱う |
| **優先度** | 低（現状 60 FPS が維持できていれば後回し） |

---

## STEPS_PERF との対応表

| PRIORITY_STEPS | STEPS_PERF | 備考 |
|----------------|------------|------|
| P1 | Step 29.1, 29.2 | Lightning チェーンは P1 で追加タスクとして記載 |
| P2 | Step 29.3 | |
| P3 | Step 28 | |
| P4 | Step 31 | オプション |
| P5 | Step 26 | |
| P6 | Step 27 | |
| P7 | Step 30 | |
| G1 | — | 新規（STEPS_PERF に未記載） |
| G2 | — | 新規 |
| G3 | — | 新規 |
| Q1 | — | 新規 |
| Q2 | — | 将来検討 |

---

## 推奨実施順序（要約）

```
1. P1  (Spatial Hash 最近接 + Lightning)  ← 最大のボトルネック解消
2. P2  (RwLock)
3. P3  (フリーリスト)
4. P5  (イベントバス)                    ← Elixir 拡張の基盤
5. P6  (ETS キャッシュ + 入力ポーリング)
6. P7  (Telemetry)
7. G1  (main/lib 統合)                   ← 汎用化の土台
8. Q1  (テストコード)
9. G2  (シーン管理)
10. G3 (アセット管理)
... P4 (SIMD), Q2 (NIF) は必要に応じて
```

---

## 関連ドキュメント

- [ENGINE_ANALYSIS.md](./ENGINE_ANALYSIS.md) — 強み・弱みの分析（本ロードマップの根拠）
- [STEPS_PERF.md](./STEPS_PERF.md) — パフォーマンス改善の詳細実装手順
- [STEPS.md](./STEPS.md) — 初回実装ステップ（Step 1〜25）
- [SPEC.md](./SPEC.md) — ゲーム仕様書
