# Path: umbrella/apps/game_network/lib/game_network/channels/user_channel.ex
# Summary: ユーザー Channel（メッセージ・通知・フレンドイベント）
defmodule GameNetwork.UserChannel do
  use Phoenix.Channel

  def join("user:" <> user_id, _params, socket) do
    if socket.assigns.user_id == user_id do
      {:ok, socket}
    else
      {:error, %{reason: "unauthorized"}}
    end
  end

  def handle_in("ping", _payload, socket) do
    {:reply, {:ok, %{pong: true}}, socket}
  end
end
