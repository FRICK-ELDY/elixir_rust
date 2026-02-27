# ADR: Elixir SSOT + Push 型同期 + スレッドポリシー

- Status: Accepted
- Date: 2026-02-27
- Supersedes: ADR_SHARED_MEMORY_THREAD_POLICY (2026-02-26)
- Scope: `native/game_native` `lib/engine` `lib/app`

## Context

前 ADR（2026-02-26）では Rust 側 `GameWorld` をゲーム状態の実体（SSOT）とし、
Elixir は `Resource` ハンドル参照のみを保持する方針を採用していた。

この方針では以下の課題が生じる：

- Elixir 側でゲームロジックを差し替えたり、サーバーにデプロイしたりする際に
  Rust 依存が強く、Elixir 単体での動作が困難になる。
- 状態の「正」が Rust にあるため、Elixir の監視・復旧・ルーム管理の強みを活かしにくい。

一方で、高頻度処理（物理・描画・音）を Elixir で担うと 60fps 維持が難しい事実は変わらない。

## Decision

### 1. Elixir を SSOT（Single Source of Truth）とする

- ゲーム状態の「正」は Elixir 側（`GameEvents` GenServer 等）が保持する。
- Elixir のティックレートは **設定可能**とし、実行環境・ゲームジャンル・サーバー負荷に応じて選択する。
  - 設定キー: `tick_hz`（起動時または実行時に変更可）

| `tick_hz` | 間隔 | 用途 |
|-----------|------|------|
| `10` | 100ms | サーバー負荷を抑えたい場合・デバッグ・ヘッドレス運用 |
| `20` | 50ms | **デフォルト**（バレットヘル・サバイバー系） |
| `30` | 33ms | よりレスポンスが必要なゲーム・将来の高精度モード |

- Rust 側は「計算・描画・音の実行エンジン」として動作し、状態の実体を持たない。

### 2. 同期モデル：Push 型（Elixir → Rust、可変 Hz）

Elixir が毎ティック（設定 Hz）Rust に **軽量制御データ** を push する。

```
Elixir tick（tick_hz: 10 / 20 / 30、設定値に従う）
  1. ゲームロジック更新（状態遷移・ルール・入力反映）
  2. control: push_tick(inputs, commands) → Rust NIF へ送信
       ※ inputs:   プレイヤー入力・移動方向など（軽量）
       ※ commands: spawn_enemy / pause / resume など制御コマンド（必要時のみ）
       ※ 全エンティティの位置・HP 等の完全状態は送らない
  3. Rust から physics_result(delta) を受け取る
       ※ delta: 位置・HP・イベント等の変化分のみ（全量ではない）
  4. Elixir SSOT に delta を適用して更新
```

- Rust は受け取った入力・コマンドを元に物理計算を実行し、**変化分（delta）** を返す。
- 描画スレッドは Rust 内部の最新状態を **60Hz に補間**して描画する（ティックレートに依存しない）。
- Elixir が Rust なしでも（ヘッドレスで）動作できる構造を維持する。

> **「スナップショット」という語について**  
> 本 ADR での「push するスナップショット」は「全状態の複製」ではなく、  
> 「その tick に必要な軽量制御データ（入力 + コマンド）」を指す。  
> 全状態の転送は `snapshot_heavy`（セーブ・デバッグ用）にのみ許可する。

### 3. スレッドモデル

Rust 側は 3 つのスレッドが独立して 60Hz で動作する。

| スレッド | 役割 | 状態への関与 |
|----------|------|-------------|
| 計算スレッド | physics_step, AI, collision | Elixir から受け取った入力・コマンドを元に計算し、delta を返す |
| 描画スレッド | wgpu render（60Hz 補間） | Rust 内部の最新状態を補間して描画。状態を変更しない |
| 音スレッド | rodio audio（60Hz） | コマンド駆動。状態を直接変更しない |

### 4. NIF API 分類（3 区分維持・方向統一）

| 区分 | 方向 | 用途 | 頻度 |
|------|------|------|------|
| `control` | **Elixir → Rust** | 入力・制御コマンドの push（`push_tick`） | 毎ティック（10 / 20 / 30Hz、設定値に従う） |
| `query_light` | Rust → Elixir | 軽量メタデータ（HP・スコア・敵数等） | 毎フレーム可 |
| `snapshot_heavy` | 双方向 | セーブ・ロード・デバッグ | 明示操作時のみ |

- `control` の方向は **Elixir → Rust への一方向** に統一する。
- `control` 以外で Rust 側の状態を変更する入口は増やさない。

## Consequences

- Elixir が SSOT になることで、監視・復旧・ルーム管理の強みを最大限に活かせる。
- Elixir 部分のみを Phoenix サーバーにデプロイできる（Rust は描画・音のクライアント専用）。
- 描画スレッドの 60Hz 補間により、10〜30Hz 更新でも滑らかな描画を維持できる。
- NIF 境界を通過するデータは「入力・コマンド（push_tick）」と「変化分 delta（physics_result）」に限定するため、
  毎フレームの全状態転送と比べて NIF 往復コストを大幅に抑えられる。
- `tick_hz` を下げるほどサーバー負荷・NIF 往復コストが減り、上げるほどゲームの応答性が増す。

## Guardrails

- `control` は Elixir → Rust の一方向のみ。Rust から Elixir への状態 push は行わない。
- `push_tick` で送るデータは **入力（inputs）と制御コマンド（commands）のみ**。全エンティティの状態を毎ティック転送しない。
- `physics_result` で返すデータは **変化分（delta）のみ**。変化のないエンティティのデータは含めない。
- 全状態の転送（全エンティティの位置・HP 等）は `snapshot_heavy` にのみ許可し、明示操作時（セーブ・ロード・デバッグ）に限定する。
- lock 競合は計測し、閾値超過時に警告を出す（詳細は実装メトリクスに従う）。
- サーバーデプロイ時は Rust スレッドを起動しない（Elixir のみで動作する）構成を維持する。
