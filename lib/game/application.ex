defmodule Game.Application do
  use Application

  @impl true
  def start(_type, _args) do
    children = [
      Game.InputHandler,
      Game.GameLoop,
    ]

    opts = [strategy: :one_for_one, name: Game.Supervisor]
    Supervisor.start_link(children, opts)
  end
end
