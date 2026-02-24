# なぜ Telemetry をメトリクス基盤に選ぶのか

このドキュメントでは、Elixir 側のメトリクス・イベント計測基盤として Telemetry を採用した技術的根拠を説明します。

---

## 目次

1. [Telemetry とは](#1-telemetry-とは)
2. [採用理由](#2-採用理由)
3. [本プロジェクトでの利用](#3-本プロジェクトでの利用)
4. [関連ドキュメント](#4-関連ドキュメント)

---

## 1. Telemetry とは

Telemetry は、Phoenix・Ecto など Elixir エコシステムで標準的に使われるメトリクス基盤です。

- **パッケージ**: telemetry ~> 1.3、telemetry_metrics ~> 1.0（mix.exs）
- **役割**: イベント発火とハンドラ登録による疎結合な計測
- **TelemetryMetrics**: Telemetry イベントから Prometheus 形式などのメトリクスを集計

---

## 2. 採用理由

### 2.1 エコシステムの標準

Phoenix、Ecto、Plug など多くのライブラリが Telemetry を採用しています。ゲームでも同じ基盤を使うことで、将来 Web ダッシュボードや Phoenix 連携を行う際に自然に統合できます。

### 2.2 疎結合な設計

計測したいコードは `:telemetry.execute/3` を呼ぶだけでよく、ハンドラの登録・変更は計測側と独立しています。計測を無効化・変更してもビジネスロジックに手を入れずに済みます。

### 2.3 低オーバーヘッド

イベント発火は軽量です。ハンドラが登録されていなければ、オーバーヘッドはほとんど発生しません。

### 2.4 拡張性

- **Prometheus**: telemetry_metrics + prom_ex でメトリクスエクスポート
- **LiveDashboard**: Phoenix LiveDashboard と連携可能
- **カスタムハンドラ**: ログ出力、アラート、外部サービス送信など自由に追加可能

---

## 3. 本プロジェクトでの利用

ゲームのパフォーマンス計測に利用しています。

| イベント | 用途 |
|----------|------|
| `[:game, :tick]` | フレームごとの physics_ms、enemy_count など |
| `[:game, :session_end]` | ゲームオーバー時の elapsed_seconds、score |
| `[:game, :level_up]` | レベルアップ時の level |
| `[:game, :boss_spawn]` | ボス出現時 |

`Engine.Telemetry` がハンドラを登録し、将来のダッシュボード・可視化に備えています。

---

## 4. 関連ドキュメント

| ドキュメント | 用途 |
|-------------|------|
| [WHY_ELIXIR.md](./WHY_ELIXIR.md) | Elixir 採用の技術的根拠。依存関係一覧あり |
