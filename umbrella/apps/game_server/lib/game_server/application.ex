# Path: umbrella/apps/game_server/lib/game_server/application.ex
# Summary: 本番サーバーデプロイ用エントリ Application（headless: true）
defmodule GameServer.Application do
  use Application

  @impl true
  def start(_type, _args) do
    children = []
    opts = [strategy: :one_for_one, name: GameServer.Supervisor]
    Supervisor.start_link(children, opts)
  end
end
