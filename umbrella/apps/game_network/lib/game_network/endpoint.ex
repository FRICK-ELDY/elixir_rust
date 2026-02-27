# Path: umbrella/apps/game_network/lib/game_network/endpoint.ex
# Summary: Phoenix Endpoint（WebSocket 受け付け）
defmodule GameNetwork.Endpoint do
  use Phoenix.Endpoint, otp_app: :game_network

  socket "/socket", GameNetwork.UserSocket,
    websocket: true,
    longpoll: false

  plug Plug.RequestId
  plug Plug.Telemetry, event_prefix: [:phoenix, :endpoint]
  plug Plug.Parsers,
    parsers: [:urlencoded, :multipart, :json],
    pass: ["*/*"],
    json_decoder: Phoenix.json_library()
  plug Plug.MethodOverride
  plug Plug.Head
  plug GameNetwork.Router
end
