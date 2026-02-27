# Path: umbrella/apps/game_network/lib/game_network/router.ex
# Summary: Phoenix Router（最小構成）
defmodule GameNetwork.Router do
  use Phoenix.Router, helpers: false

  pipeline :api do
    plug :accepts, ["json"]
  end

  scope "/api", GameNetwork do
    pipe_through :api
  end
end
