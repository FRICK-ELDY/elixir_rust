# Path: umbrella/apps/game_network/lib/game_network/application.ex
# Summary: game_network OTP Application（Phoenix Endpoint 起動）
defmodule GameNetwork.Application do
  use Application

  @impl true
  def start(_type, _args) do
    children = [
      GameNetwork.Endpoint
    ]

    opts = [strategy: :one_for_one, name: GameNetwork.Supervisor]
    Supervisor.start_link(children, opts)
  end
end
