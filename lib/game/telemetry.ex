defmodule Game.Telemetry do
  @moduledoc """
  Telemetry イベントのハンドラーと Metrics を定義する Supervisor。

  計測ポイント:
    [:game, :tick]          — 毎フレームの物理ステップ時間・敵数
    [:game, :level_up]      — レベルアップ発生
    [:game, :boss_spawn]    — ボス出現
    [:game, :session_end]   — ゲームオーバー

  将来的な拡張:
    - Phoenix LiveDashboard との接続
    - Prometheus / Grafana へのエクスポート
    - ゲームセッションのリプレイ記録
  """

  use Supervisor

  def start_link(opts), do: Supervisor.start_link(__MODULE__, opts, name: __MODULE__)

  @impl true
  def init(_opts) do
    children = [
      {Telemetry.Metrics.ConsoleReporter, metrics: metrics()},
    ]
    Supervisor.init(children, strategy: :one_for_one)
  end

  def metrics do
    [
      # physics_ms: summary で min/max/mean/percentiles を集計（パフォーマンス分析用）
      Telemetry.Metrics.summary("game.tick.physics_ms",
        unit: :millisecond,
        description: "Rust physics step duration per frame"
      ),
      # enemy_count: last_value で現在値、summary で平均・最大値・パーセンタイルを集計
      Telemetry.Metrics.last_value("game.tick.enemy_count",
        description: "Active enemy count (current)"
      ),
      Telemetry.Metrics.summary("game.tick.enemy_count",
        description: "Active enemy count (avg/max/percentiles over report period)"
      ),
      Telemetry.Metrics.counter("game.level_up.count",
        description: "Total level-up events"
      ),
      Telemetry.Metrics.counter("game.boss_spawn.count",
        description: "Total boss spawn events"
      ),
    ]
  end
end
