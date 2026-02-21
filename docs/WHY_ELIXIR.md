# なぜ Elixir をゲームの司令塔に選ぶのか

このドキュメントでは、ヴァンパイアサバイバーライクゲームのゲームロジック層に Elixir（BEAM VM）を採用した技術的根拠を説明します。

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

Elixir の `GenServer` は、ゲームループを**状態機械**として自然に表現できます。

```elixir
defmodule Game.GameLoop do
  use GenServer

  # 状態遷移が明示的で追跡しやすい
  def handle_info(:tick, %{phase: :playing} = state) do
    # ゲーム中の処理
    {:noreply, update_game(state)}
  end

  def handle_info(:tick, %{phase: :paused} = state) do
    # ポーズ中は何もしない
    {:noreply, state}
  end

  def handle_info(:tick, %{phase: :game_over} = state) do
    # ゲームオーバー処理
    {:noreply, handle_game_over(state)}
  end
end
```

従来の命令型言語では `if/switch` の連鎖になりがちなゲームフェーズ管理が、パターンマッチングにより**読みやすく、バグが混入しにくい**コードになります。

### メッセージパッシングによる疎結合

各ゲームシステムが独立したプロセスとして動作し、メッセージで通信します。

```
InputHandler ──cast──▶ GameLoop ──call──▶ NIF (Rust)
                           │
                           └──cast──▶ SpawnSystem
                           └──cast──▶ ScoreSystem
                           └──cast──▶ AudioSystem
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

ただし、Rust NIF の呼び出しは `dirty_cpu` スケジューラを使用することで、BEAM スケジューラをブロックしない設計にしています（詳細は [SPEC.md](SPEC.md) 参照）。

---

## 4. 耐障害性とクラッシュリカバリ

### Supervisor ツリーによる自動回復

```elixir
defmodule Game.Application do
  use Application

  def start(_type, _args) do
    children = [
      # 各システムが独立して監視される
      {Game.ComponentStore,  restart: :permanent},
      {Game.InputHandler,    restart: :permanent},
      {Game.SpawnSystem,     restart: :transient},  # 異常終了時のみ再起動
      {Game.ScoreSystem,     restart: :permanent},
      {Game.GameLoop,        restart: :permanent},
    ]
    Supervisor.start_link(children, strategy: :one_for_one)
  end
end
```

`ScoreSystem` がバグでクラッシュしても、`GameLoop` は継続して動作します。Supervisor が `ScoreSystem` を自動再起動し、ゲームは中断なく続きます。

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

ETS は BEAM VM に組み込まれたインメモリ Key-Value ストアです。ゲームの ECS コンポーネントストアとして最適な特性を持ちます。

| 特性 | 詳細 |
|---|---|
| 読み取り速度 | O(1)、複数プロセスが並行読み取り可能 |
| 書き込み速度 | O(1)、ロックフリー（`:public` テーブル） |
| データ量 | 数百万エントリまで実用的 |
| 永続性 | なし（プロセス終了で消滅）→ ゲームに最適 |
| GC 影響 | なし（BEAM GC の対象外） |

### ゲームコンポーネントストアとしての活用

```elixir
defmodule Game.ComponentStore do
  def setup do
    # コンポーネント種別ごとにテーブルを作成
    :ets.new(:positions,  [:named_table, :public, :set, read_concurrency: true])
    :ets.new(:health,     [:named_table, :public, :set, read_concurrency: true])
    :ets.new(:sprites,    [:named_table, :public, :set, read_concurrency: true])
  end

  # Rust から受け取った一括更新結果を書き込む
  def bulk_update_positions(entities) do
    :ets.insert(:positions, entities)  # リスト一括挿入
  end

  # UI システムが非同期で読み取る
  def get_player_position do
    :ets.lookup(:positions, :player)
  end
end
```

### ETS vs 代替手段

| 手段 | 読み取り速度 | 並行性 | 適用場面 |
|---|---|---|---|
| ETS | 最高速 | 並行読み取り可 | 頻繁に更新・参照されるゲームステート |
| GenServer state | 遅い（メッセージ経由） | 直列 | ゲームフェーズ等の重要な状態 |
| Agent | 遅い（メッセージ経由） | 直列 | 単純な設定値 |
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

Rustler を使うと、Rust の関数が Elixir から**通常の関数呼び出しと同じ構文**で呼べます。

```elixir
# Elixir から見ると普通の関数呼び出し
render_data = Game.NifBridge.physics_step(world_ref, 16.0)

# 実際には Rust のネイティブコードが実行される
# → 物理演算・衝突判定・AI 更新が全て Rust で処理
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
  2. GenServer による宣言的なゲームループ → 複雑なゲームロジックの管理
  3. Rustler NIF による Rust との透過的な統合 → 性能と生産性の両立
  4. 将来的な分散処理（マルチプレイ）への自然な拡張
  5. ETS による高速なコンポーネントストア
```

---

## まとめ

Elixir は「ゲームエンジン」ではありませんが、「**ゲームの司令塔**」として優れた特性を持ちます。

- **BEAM VM** の並行性・耐障害性・ソフトリアルタイム性はゲームロジック層に最適
- **Rust** の性能・メモリ制御・GPU アクセスは描画・物理演算層に最適
- **Rustler NIF** がこの 2 つを透過的に結合する

この組み合わせは「Elixir の生産性と Rust の性能」を同時に実現する、現時点での最良のアーキテクチャの一つです。
