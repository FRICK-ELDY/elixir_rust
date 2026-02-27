# Path: umbrella/apps/game_network/lib/game_network/channels/lobby_channel.ex
# Summary: ロビー Channel（Presence・オンライン一覧）
defmodule GameNetwork.LobbyChannel do
  use Phoenix.Channel

  alias GameNetwork.Presence

  def join("lobby", _params, socket) do
    send(self(), :after_join)
    {:ok, socket}
  end

  def handle_info(:after_join, socket) do
    {:ok, _} =
      Presence.track(socket, socket.assigns.user_id, %{
        online_at: System.system_time(:second)
      })

    push(socket, "presence_state", Presence.list(socket))
    {:noreply, socket}
  end

  def handle_in("ping", _payload, socket) do
    {:reply, {:ok, %{pong: true}}, socket}
  end
end
