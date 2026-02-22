defmodule Game.Application do
  use Application

  @impl true
  def start(_type, _args) do
    children = [
      # Step 7 以降でここに GameLoop などを追加する
    ]

    opts = [strategy: :one_for_one, name: Game.Supervisor]
    Supervisor.start_link(children, opts)
  end
end
