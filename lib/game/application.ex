defmodule Game.Application do
  use Application

  @impl true
  def start(_type, _args) do
    children = [
      # G2: シーン管理 — GameLoop より前に起動
      Engine.SceneManager,
      # Input handler: translates key events to GameLoop casts
      Engine.InputHandler,
      # Step 26: イベントバス — GameLoop より前に起動
      Engine.EventBus,
      # Core game loop: 60 Hz physics tick via Rust NIF
      Engine.GameLoop,
      # Independent performance monitor
      Engine.StressMonitor,
      # Step 25: ゲームセッション統計収集
      Engine.Stats,
      # P7: Telemetry 計測基盤
      Engine.Telemetry,
    ]

    opts = [strategy: :one_for_one, name: Game.Supervisor]
    Supervisor.start_link(children, opts)
  end
end
