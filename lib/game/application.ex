defmodule Game.Application do
  use Application

  @impl true
  def start(_type, _args) do
    children = [
      # Input handler: translates key events to GameLoop casts
      Game.InputHandler,
      # Core game loop: 60 Hz physics tick via Rust NIF
      Game.GameLoop,
      # Independent performance monitor: samples every second.
      # Demonstrates OTP: this process is completely isolated from the game loop.
      # A crash here never affects gameplay; the supervisor restarts it automatically.
      Game.StressMonitor,
      # Step 25: ゲームセッション統計収集（Elixir の強みを活かした独立プロセス）
      Game.Stats,
    ]

    opts = [strategy: :one_for_one, name: Game.Supervisor]
    Supervisor.start_link(children, opts)
  end
end
