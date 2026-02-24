# なぜ BEAM 組み込み機能を活用するのか

このドキュメントでは、Elixir（BEAM VM）の標準ライブラリ・組み込み機能をゲーム開発でどのように活用するかを説明します。追加パッケージなしで利用できる機能です。

---

## 目次

1. [BEAM 組み込みとは](#1-beam-組み込みとは)
2. [GenServer](#2-genserver)
3. [Supervisor](#3-supervisor)
4. [ETS](#4-ets)
5. [Process](#5-process)
6. [Logger](#6-logger)
7. [関連ドキュメント](#7-関連ドキュメント)

---

## 1. BEAM 組み込みとは

BEAM VM に組み込まれた機能は、依存関係を追加せずに利用できます。本プロジェクトでは以下の機能を活用しています。

| 機能 | 用途 |
|------|------|
| **GenServer** | ゲームループ・状態機械・各種サブシステム |
| **Supervisor** | 耐障害性・自動再起動 |
| **ETS** | 入力状態・HUD スナップショット等の高速共有 |
| **Process** | メッセージパッシング・スケジューリング |
| **Logger** | ログ出力 |
| **Registry** | ルーム ID → GameEvents pid の名前解決 |

---

## 2. GenServer

ゲームループ（`Engine.GameEvents`）、シーン管理（`Engine.SceneManager`）、入力（`Engine.InputHandler`）、イベント配信（`Engine.EventBus`）など、状態を持つプロセスは GenServer で実装しています。

状態機械としてのゲームフェーズ管理、メッセージによる疎結合な通信が自然に表現できます。詳細は [WHY_ELIXIR.md](./WHY_ELIXIR.md) §2 を参照してください。

---

## 3. Supervisor

`App.Application` 配下で `one_for_one` 戦略の Supervisor を構成しています。各子プロセスが独立して監視され、クラッシュ時には該当プロセスのみが再起動されます。

「Let it crash」哲学に基づき、防御的プログラミングを減らしつつ耐障害性を確保しています。詳細は [WHY_ELIXIR.md](./WHY_ELIXIR.md) §4 を参照してください。

---

## 4. ETS

インメモリ Key-Value ストア。本プロジェクトでは以下の用途で使用しています。

| テーブル | 用途 | ライター | リーダー |
|----------|------|----------|----------|
| `:input_state` | キー入力状態（move vector） | InputHandler | GameEvents |
| `:frame_cache` | HUD スナップショット（敵数、physics_ms 等） | GameEvents | StressMonitor、Rust 描画 |

`read_concurrency: true` により並列読み取りを最適化しています。ゲーム状態（位置・体力・スコア）は Rust の `world_ref` 内に保持し、Elixir は NIF 経由で参照します。詳細は [WHY_ELIXIR.md](./WHY_ELIXIR.md) §5 を参照してください。

---

## 5. Process

メッセージパッシング（`send`/`receive`）、`Process.send_after/3` による遅延実行、`Process.monitor/1` によるプロセス監視など、BEAM のプロセスモデルを活用しています。

本プロジェクトでは tick の主導権は Rust にあり、Elixir は `{:frame_events, events}` を受信してイベント駆動で応答します。`Process.send_after` は他用途（タイムアウト、遅延実行など）で利用可能です。

---

## 6. Logger

標準の `Logger` でゲームログ、デバッグ情報、エラーを出力しています。開発・本番でログレベルの切り替えが容易です。

---

## 7. 関連ドキュメント

| ドキュメント | 用途 |
|-------------|------|
| [WHY_ELIXIR.md](./WHY_ELIXIR.md) | Elixir 採用の技術的根拠。依存関係一覧あり |
