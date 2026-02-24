defmodule Engine.RoomRegistry do
  @moduledoc """
  Step 44: ルーム ID → GameLoop pid のマッピング用 Registry。

  複数ルーム運用時、`Engine.RoomSupervisor` が起動した GameLoop を
  この Registry に登録する。Phoenix Channel 等からルーム指定で
  GameLoop にアクセスする際に使用する。

  ## 利用例

      # ルームの GameLoop pid を取得
      {:ok, pid} = Engine.RoomRegistry.get_loop(:main)

      # 登録済みルーム一覧
      Engine.RoomRegistry.list_rooms()
  """

  @registry __MODULE__

  @doc """
  ルーム ID に対応する GameLoop の pid を返す。

  ## 例
      Engine.RoomRegistry.get_loop(:main)
      Engine.RoomRegistry.get_loop("room_123")
  """
  def get_loop(room_id) when is_binary(room_id) or is_atom(room_id) do
    case Registry.lookup(@registry, room_id) do
      [{pid, _}] when is_pid(pid) -> {:ok, pid}
      [] -> :error
    end
  end

  @doc """
  登録済みの全ルーム ID のリストを返す。
  """
  def list_rooms do
    @registry
    |> Registry.select([{{:"$1", :_, :_}, [], [:"$1"]}])
  end

  @doc """
  Registry に呼び出し元プロセスを room_id で登録する。GameLoop の init から呼ばれる。
  """
  def register(room_id) when is_binary(room_id) or is_atom(room_id) do
    case Registry.register(@registry, room_id, []) do
      :ok -> :ok
      {:error, {:already_registered, _pid}} -> :ok
      other -> other
    end
  end

  @doc "Registry から room_id の登録を解除する。GameLoop 終了時に呼ばれる。"
  def unregister(room_id) when is_binary(room_id) or is_atom(room_id) do
    Registry.unregister(@registry, room_id)
  end
end
