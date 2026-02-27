# Path: umbrella/apps/game_network/lib/game_network/user_socket.ex
# Summary: Phoenix Socket（user_id 認証・Channel ルーティング）
defmodule GameNetwork.UserSocket do
  use Phoenix.Socket

  channel "user:*", GameNetwork.UserChannel
  channel "room:*", GameNetwork.RoomChannel
  channel "lobby", GameNetwork.LobbyChannel

  @impl true
  def connect(%{"token" => token}, socket, _connect_info) do
    case verify_token(token) do
      {:ok, user_id} ->
        {:ok, assign(socket, :user_id, user_id)}

      {:error, _reason} ->
        :error
    end
  end

  def connect(_params, _socket, _connect_info), do: :error

  @impl true
  def id(socket), do: "user_socket:#{socket.assigns.user_id}"

  defp verify_token(token) do
    Phoenix.Token.verify(GameNetwork.Endpoint, "user socket", token, max_age: 86_400)
  end
end
