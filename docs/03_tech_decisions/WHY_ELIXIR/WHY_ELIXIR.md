# なぜ Elixir をゲームの司令塔に選ぶのか

このドキュメントでは、ヴァンパイアサバイバーライクゲームのゲームロジック層に Elixir（BEAM VM）を採用した技術的根拠を説明します。

### 依存関係の詳細と一覧

| パッケージ | バージョン | 用途 | 詳細 |
|------------|------------|------|------|
| rustler | ~> 0.34 | Rust NIF 連携（Elixir-Rust ブリッジ） | [WHY_Rustler.md](../WHY_Rustler.md) |
| telemetry | ~> 1.3 | メトリクス・イベント計測基盤 | [WHY_Telemetry.md](./WHY_Telemetry.md) |
| telemetry_metrics | ~> 1.0 | メトリクス集計・可視化 | [WHY_Telemetry.md](./WHY_Telemetry.md) |
| BEAM 組み込み | 標準 | GenServer、Supervisor、ETS、Process、Logger | [WHY_BEAM.md](./WHY_BEAM.md) |

---

## 目次

1. [BEAM VM の設計思想](#1-beam-vm-の設計思想)
2. [ゲームループへの適合性](#2-ゲームループへの適合性)
3. [並行性モデルの優位性](#3-並行性モデルの優位性)
4. [耐障害性とクラッシュリカバリ](#4-耐障害性とクラッシュリカバリ)
5. [ETS による高速共有ステート](#5-ets-による高速共有ステート)
6. [Rust との役割分担の合理性](#6-rust-との役割分担の合理性)
7. [他言語との比較](#7-他言語との比較)
8. [トレードオフと注意点](#8-トレードオフと注意点)

---

## 1. BEAM VM の設計思想

Elixir が動作する BEAM（Bogdan/Björn's Erlang Abstract Machine）は、1980年代後半に Ericsson が電話交換機向けに開発した仮想マシンです。その設計目標は現代のゲーム開発が求める要件と驚くほど一致しています。

| BEAM の設計目標 | ゲーム開発での対応 |
|---|---|
| **高並行性**（数百万プロセス） | 多数のゲームシステムを独立して動かす |
| **耐障害性**（99.9999999% 稼働率） | ゲームクラッシュからの自動回復 |
| **ソフトリアルタイム性** | 安定した 60fps ゲームループ |
| **ホットコードスワップ** | 実行中のゲームロジック変更 |
| **分散処理** | 将来的なマルチプレイ対応 |

BEAM は「壊れることを前提に設計する（Let it crash）」という哲学を持ちます。これはゲーム開発においても有効で、バグが発生しても**ゲーム全体を落とさずに問題のあるシステムだけを再起動**できます。

---

## 2. ゲームループへの適合性

### GenServer による宣言的なゲームループ

Elixir の `GenServer` は、ゲームループを**状態機械**として自然に表現できます。本プロジェクトでは **Rust が tick を主導**し、物理演算後に Elixir に `{:frame_events, events}` を送信します。

```elixir
defmodule Engine.GameLoop do
  use GenServer

  # Rust が物理演算を実行後に送るイベントを受信
  def handle_info({:frame_events, events}, state) do
    # 1. 入力取得（ETS）→ set_player_input NIF
    # 2. EventBus.broadcast(events)
    # 3. 現在シーンの update(context, scene_state) を呼び出し
    # 4. {:continue, ...} / {:transition, :push|:pop|:replace, ...} で遷移を宣言
    {:noreply, new_state}
  end
end
```

フェーズ（`:playing` / `:boss_alert` / `:level_up` / `:game_over`）は `SceneManager` と各シーンで管理され、パターンマッチングにより**読みやすく、バグが混入しにくい**コードになっています。

### メッセージパッシングによる疎結合

各ゲームシステムが独立したプロセスとして動作し、メッセージまたは ETS で通信します。

```
Rust (physics) ──{:frame_events}──▶ Engine.GameLoop
                                         │
InputHandler (ETS) ◀──get_move_vector──  │  set_player_input(world_ref) ──▶ NIF
                                         │
                                         └──EventBus.broadcast(events)──▶ サブスクライバー
                                         │
SceneManager (current scene) ◀──update(context)──│
     │
     └── Playing ──SpawnSystem.maybe_spawn(world_ref)──▶ Engine.spawn_enemies (NIF)
```

システム間の依存関係が明確になり、**テストが容易**で**システムの追加・削除が安全**です。

---

## 3. 並行性モデルの優位性

### プロセスの軽量さ

BEAM のプロセスは OS スレッドではなく、VM が管理する**グリーンスレッド**です。

| | BEAM プロセス | OS スレッド |
|---|---|---|
| 生成コスト | ~1μs、約 2KB | ~10ms、~8MB |
| 同時実行数 | 数百万 | 数千（OS 制限） |
| スケジューリング | プリエンプティブ（VM管理） | OS 管理 |
| 通信 | メッセージパッシング（コピー） | 共有メモリ（ロック必要） |

ゲームの各サブシステム（スポーン・スコア・オーディオ・ネットワーク）を独立したプロセスとして動かしても、オーバーヘッドはほぼゼロです。

### プリエンプティブスケジューリングによる安定フレームレート

BEAM のスケジューラは**リダクション（命令数）ベースのプリエンプション**を行います。1プロセスが長時間CPUを占有することを防ぎ、ゲームループの tick が他の処理によって遅延するリスクを低減します。

```
従来のシングルスレッドゲームループ:
[tick1][重い処理...........][tick2]  ← tick2 が遅延

BEAM VM のスケジューリング:
[tick1][重い処理の一部][他プロセス][重い処理の続き][tick2]  ← tick2 は定時
```

ただし、Rust NIF の呼び出しは `dirty_cpu` スケジューラを使用することで、BEAM スケジューラをブロックしない設計にしています（詳細は [SPEC.md](../../01_setup/SPEC.md) 参照）。

---

## 4. 耐障害性とクラッシュリカバリ

### Supervisor ツリーによる自動回復

```elixir
defmodule App.Application do
  use Application

  def start(_type, _args) do
    children = [
      {Registry, [keys: :unique, name: Engine.RoomRegistry]},
      Engine.SceneManager,
      Engine.InputHandler,
      Engine.EventBus,
      Engine.RoomSupervisor,   # GameLoop をルーム単位で起動
      Engine.StressMonitor,
      Engine.Stats,
      Engine.Telemetry,
    ]
    Supervisor.start_link(children, strategy: :one_for_one, name: App.Supervisor)
  end
end
```

`EventBus` や `InputHandler` がバグでクラッシュしても、`GameLoop` は継続して動作します。Supervisor が該当プロセスを自動再起動し、ゲームは中断なく続きます。`GameLoop` 自体は `RoomSupervisor` 配下でルームごとに起動されます。

### 「Let it crash」哲学の実践

防御的プログラミング（大量の `try/catch`）の代わりに、**異常系は Supervisor に任せる**という設計です。

```elixir
# 悪い例: 防御的すぎるコード
def handle_info(:tick, state) do
  try do
    result = do_something_risky(state)
    {:noreply, result}
  rescue
    e ->
      Logger.error("Error: #{inspect(e)}")
      {:noreply, state}  # エラーを隠蔽してしまう
  end
end

# 良い例: クラッシュさせて Supervisor に再起動させる
def handle_info(:tick, state) do
  result = do_something_risky(state)  # クラッシュしたら Supervisor が再起動
  {:noreply, result}
end
```

これにより、**エラーハンドリングのコードが大幅に削減**され、バグの発見も容易になります。

---

## 5. ETS による高速共有ステート

### ETS（Erlang Term Storage）とは

ETS は BEAM VM に組み込まれたインメモリ Key-Value ストアです。ゲームの高頻度データ共有に最適な特性を持ちます。

| 特性 | 詳細 |
|---|---|
| 読み取り速度 | O(1)、複数プロセスが並行読み取り可能 |
| 書き込み速度 | O(1)、ロックフリー（`:public` テーブル） |
| データ量 | 数百万エントリまで実用的 |
| 永続性 | なし（プロセス終了で消滅）→ ゲームに最適 |
| GC 影響 | なし（BEAM GC の対象外） |

### 本プロジェクトでの ETS 活用

**ゲーム状態（位置・体力など）は Rust の `world_ref` 内に保持**し、Elixir は NIF 経由で参照します。ETS は以下の用途で使用しています。

**InputHandler（入力状態）**

```elixir
defmodule Engine.InputHandler do
  @table :input_state

  # GameLoop が tick ごとにロックフリーで読み取る
  def get_move_vector do
    case :ets.lookup(@table, :move) do
      [{:move, vec}] -> vec
      []             -> {0, 0}
    end
  end

  def init(_opts) do
    :ets.new(@table, [:named_table, :public, :set, read_concurrency: true])
    :ets.insert(@table, {:move, {0, 0}})
    # ...
  end
end
```

**FrameCache（HUD スナップショット）**

```elixir
# GameLoop が NIF から get_frame_metadata で取得した値を書き込み
# Rust 側の描画や StressMonitor がロックフリーで読み取る
Engine.FrameCache.put(enemy_count, bullet_count, physics_ms, hud_data, render_type)
```

### ETS vs 代替手段

| 手段 | 読み取り速度 | 並行性 | 適用場面 |
|---|---|---|---|
| ETS | 最高速 | 並行読み取り可 | 入力状態、HUD スナップショット等の高頻度参照 |
| GenServer state | 遅い（メッセージ経由） | 直列 | SceneManager、シーンスタック等 |
| Rust world_ref | NIF 経由 | 単一 | 位置・体力・スコア等のゲームロジック本体 |
| Redis | ネットワーク遅延あり | 並行可 | 分散環境のみ |

---

## 6. Rust との役割分担の合理性

### 「得意なことを得意な言語に任せる」原則

```
Elixir が得意なこと:
  ✓ 並行プロセス管理
  ✓ 状態機械（GenServer）
  ✓ 宣言的なビジネスロジック
  ✓ 耐障害性・自動回復
  ✓ ホットコードスワップ
  ✗ 数値計算（GC の影響あり）
  ✗ メモリレイアウト制御
  ✗ GPU 操作

Rust が得意なこと:
  ✓ ゼロコスト抽象化
  ✓ SIMD・キャッシュ効率の最適化
  ✓ GPU 操作（wgpu）
  ✓ メモリレイアウト制御（SoA）
  ✗ 並行プロセス管理（複雑）
  ✗ 動的なゲームロジック変更
  ✗ 耐障害性の仕組み（自前実装が必要）
```

### Rustler NIF による透過的な統合

Rustler を使うと、Rust の関数が Elixir から**通常の関数呼び出しと同じ構文**で呼べます。ゲームは `Engine` モジュール経由で NIF を利用します。

```elixir
# Engine が App.NifBridge をラップ。ゲームからは Engine 経由で利用
Engine.spawn_enemies(world_ref, :slime, 5)
Engine.get_level_up_data(world_ref)
Engine.set_player_input(world_ref, dx, dy)  # GameLoop が InputHandler の結果を渡す

# 物理演算は Rust がループ内で実行し、{:frame_events, events} を GameLoop に送信
# Elixir は physics_step を直接呼ばない（Rust 駆動）
```

### ResourceArc によるゼロコピー連携

ゲームワールドの大量データを毎フレーム Elixir-Rust 間でコピーするのは非効率です。`ResourceArc` を使うと、Rust のデータを Elixir から**参照（不透明なポインタ）**として扱えます。

```
従来のアプローチ（非効率）:
  Elixir → [5000体分のデータをシリアライズ] → Rust → [処理] → [5000体分をデシリアライズ] → Elixir

ResourceArc アプローチ（効率的）:
  Elixir → [world_ref（8バイトのポインタ）] → Rust → [処理] → [50KBのレンダリングデータのみ] → Elixir
```

---

## 7. 他言語との比較

### ゲームロジック層の言語比較

| 観点 | Elixir | Go | Python | Node.js | Java/Kotlin |
|---|---|---|---|---|---|
| 並行性モデル | グリーンプロセス（数百万） | goroutine（数百万） | GIL あり（実質シングル） | イベントループ（シングル） | OS スレッド（数千） |
| 耐障害性 | Supervisor ツリー（組み込み） | 手動実装 | 手動実装 | 手動実装 | 手動実装 |
| GC 停止時間 | < 1ms（プロセス単位 GC） | < 1ms | 数十ms | 数ms | 数十ms（G1GC） |
| Rust 連携 | Rustler NIF（透過的） | CGo（複雑） | ctypes/cffi | N-API | JNI（複雑） |
| ホットリロード | ネイティブサポート | 非対応 | 非対応 | 非対応 | 非対応 |
| 学習コスト | 高（関数型パラダイム） | 低 | 最低 | 低 | 中 |
| パターンマッチング | 強力 | なし | 限定的（3.10+） | なし | なし |

### Go との比較（最も近い代替候補）

Go も goroutine による高並行性を持ちますが、ゲームロジック層としては以下の点で Elixir が優位です。

```
Go の課題:
  - Supervisor のような耐障害性の仕組みがない
  - goroutine のパニックは手動で recover する必要がある
  - ゲームフェーズ管理に状態機械ライブラリが別途必要
  - GC の Stop-the-World が稀に数ms 発生する可能性

Elixir の優位点:
  - Supervisor が自動的にクラッシュしたシステムを再起動
  - GenServer がゲームループ・状態機械を自然に表現
  - プロセス単位 GC → Stop-the-World なし
  - パターンマッチングによる宣言的なゲームロジック
```

### Python との比較

Python はゲームロジックのプロトタイピングには優れますが、本番環境での性能は課題です。

```
Python の課題:
  - GIL（Global Interpreter Lock）により真の並行処理が不可
  - GC 停止が数十ms に達することがある
  - Rust との連携（PyO3）はあるが、並行性の問題は解決しない

Elixir の優位点:
  - GIL なし、真の並行処理
  - GC 停止なし（プロセス単位 GC）
  - Rustler NIF で Rust と透過的に連携
```

---

## 8. トレードオフと注意点

### Elixir を採用することの課題

正直に言えば、Elixir にも課題はあります。

#### 学習コスト

関数型プログラミング・アクターモデル・パターンマッチングに慣れていない開発者には学習コストがかかります。特に「状態を変更する」という命令型の発想から「新しい状態を返す」という関数型の発想への転換が必要です。

```elixir
# 命令型の発想（Elixir では不可）
state.score = state.score + 100  # エラー: 変数は不変

# 関数型の発想（Elixir の正しい書き方）
new_state = %{state | score: state.score + 100}
```

#### NIF クラッシュのリスク

Rust の NIF がパニックすると、BEAM VM ごとクラッシュします（Rustler 0.29 以降は自動的に `catch_unwind` でラップされますが、未定義動作は防げません）。

```rust
// 対策: unsafe コードを最小限に、Rustler の安全機能を活用
#[rustler::nif(schedule = "DirtyCpu")]
fn physics_step(world: ResourceArc<Mutex<GameWorld>>, delta_ms: f32) -> Binary {
    // Rustler が自動的に panic を catch する（0.29+）
    // ただし unsafe ブロック内のクラッシュは防げない
}
```

#### dirty_cpu スケジューラの枯渇

`dirty_cpu` スケジューラはデフォルトで CPU コア数分しかありません。物理演算が 16ms を超えると次のフレームが詰まります。

```
対策:
  1. Rust の物理演算を確実に 8ms 以内に収める
  2. プロファイリングを定期的に実施
  3. 必要に応じて +SDcpu フラグでスケジューラ数を増やす
```

#### ゲームエンジンエコシステムの欠如

Unity・Unreal・Godot のような統合ゲームエンジンのエコシステムは Elixir にはありません。物理エンジン・アニメーション・オーディオなどは Rust 側で実装するか、外部ライブラリを組み合わせる必要があります。

### それでも Elixir を選ぶ理由

上記の課題を踏まえても、**このプロジェクトの要件**（数千体の敵・ゲームロジックの複雑さ・将来的なマルチプレイ対応）においては、Elixir の利点が課題を上回ります。

```
決定的な理由:
  1. Supervisor による耐障害性 → ゲームバグからの自動回復
  2. GenServer + SceneManager による宣言的なゲームループ → シーン遷移の管理
  3. Rustler NIF による Rust との透過的な統合 → 性能と生産性の両立
  4. 将来的な分散処理（ルーム単位の GameLoop、マルチプレイ）への自然な拡張
  5. ETS による高速なデータ共有（入力状態、HUD スナップショット）
```

---

## まとめ

Elixir は「ゲームエンジン」ではありませんが、「**ゲームの司令塔**」として優れた特性を持ちます。

- **BEAM VM** の並行性・耐障害性・ソフトリアルタイム性はゲームロジック層に最適
- **Rust** の性能・メモリ制御・GPU アクセスは描画・物理演算層に最適
- **Rustler NIF** がこの 2 つを透過的に結合する

この組み合わせは「Elixir の生産性と Rust の性能」を同時に実現する、現時点での最良のアーキテクチャの一つです。

---

## 関連ドキュメント

| ドキュメント | 用途 |
|-------------|------|
| [WHY_Rustler.md](../WHY_Rustler.md) | Rustler（NIF 連携）の選定理由 |
| [WHY_Telemetry.md](./WHY_Telemetry.md) | Telemetry の選定理由 |
| [WHY_BEAM.md](./WHY_BEAM.md) | BEAM 組み込み機能の活用 |
| [WHY_RUST/WHY_RUST.md](../WHY_RUST/WHY_RUST.md) | Rust 採用の技術的根拠 |
| [ELIXIR_RUST_DIVISION.md](../ELIXIR_RUST_DIVISION.md) | Elixir/Rust 役割分担方針 |
