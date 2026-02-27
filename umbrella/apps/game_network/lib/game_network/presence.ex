# Path: umbrella/apps/game_network/lib/game_network/presence.ex
# Summary: Phoenix.Presence（オンライン状態管理）
defmodule GameNetwork.Presence do
  use Phoenix.Presence,
    otp_app: :game_network,
    pubsub_server: GameNetwork.PubSub
end
