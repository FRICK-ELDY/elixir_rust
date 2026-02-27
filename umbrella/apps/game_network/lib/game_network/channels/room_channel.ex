# Path: umbrella/apps/game_network/lib/game_network/channels/room_channel.ex
# Summary: ゲームルーム Channel（Engine.RoomSupervisor に委譲）
defmodule GameNetwork.RoomChannel do
  use Phoenix.Channel
  require Logger

  def join("room:" <> room_id, _params, socket) do
    case GameEngine.start_room(room_id) do
      {:ok, _pid} ->
        Logger.info("[RoomChannel] Room started: #{room_id}")
        {:ok, socket}

      {:error, :already_started} ->
        {:ok, socket}

      {:error, reason} ->
        {:error, %{reason: inspect(reason)}}
    end
  end

  def handle_in("input", %{"dx" => _dx, "dy" => _dy}, socket) do
    case GameEngine.get_loop_for_room(socket.assigns[:room_id]) do
      {:ok, _pid} ->
        {:noreply, socket}

      :error ->
        {:noreply, socket}
    end
  end
end
