defmodule Engine.RoomSupervisor do
  @moduledoc """
  Step 44: ルーム単位で GameLoop を管理する DynamicSupervisor。

  各ルームは独立した GameLoop + GameWorld を持ち、互いに影響しない
  （設計オプション A: 複数 GameWorld）。

  ## 起動

  Application 起動時に `:main` ルームが自動で起動する。
  追加ルームは `start_room/1` で起動する。

  ## Phoenix Channels 連携（将来）

  `RoomChannel` で `join("room:123")` 時に `start_room("123")` を呼ぶ。
  入力イベントを Channel でブロードキャストし、各クライアントの
  GameLoop が受信する形で状態同期を行う。

  ## 例

      # ルーム起動（すでに存在する場合はエラー）
      {:ok, pid} = Engine.RoomSupervisor.start_room("room_456")

      # ルーム終了（GameWorld 解放含む）
      Engine.RoomSupervisor.stop_room("room_456")

      # アクティブなルーム一覧
      Engine.RoomSupervisor.list_rooms()
  """

  use DynamicSupervisor
  require Logger

  @default_room :main

  def start_link(opts \\ []) do
    DynamicSupervisor.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc """
  新規ルームを起動する。ルーム ID ごとに 1 つの GameLoop が起動する。
  すでに同じ room_id のルームが存在する場合は `{:error, :already_started}` を返す。
  """
  def start_room(room_id) when is_binary(room_id) or is_atom(room_id) do
    case Engine.RoomRegistry.get_loop(room_id) do
      {:ok, _pid} ->
        {:error, :already_started}

      :error ->
        child_spec =
          {Engine.GameLoop, [room_id: room_id]}
          |> Supervisor.child_spec(id: {:game_loop, room_id})

        case DynamicSupervisor.start_child(__MODULE__, child_spec) do
          {:ok, pid} ->
            Logger.info("[ROOM] Started room #{inspect(room_id)}")
            {:ok, pid}

          other ->
            other
        end
    end
  end

  @doc """
  ルームを終了する。GameLoop が停止し、GameWorld が解放される。
  """
  def stop_room(room_id) when is_binary(room_id) or is_atom(room_id) do
    case Engine.RoomRegistry.get_loop(room_id) do
      {:ok, pid} ->
        # 登録解除は GameLoop の terminate/2（:main）または Registry の自動解除（:via）で行う
        DynamicSupervisor.terminate_child(__MODULE__, pid)
        Logger.info("[ROOM] Stopped room #{inspect(room_id)}")
        :ok

      :error ->
        {:error, :not_found}
    end
  end

  @doc """
  アクティブなルーム ID のリストを返す。
  """
  def list_rooms do
    Engine.RoomRegistry.list_rooms()
  end

  @doc """
  デフォルトルーム ID（:main）。単一プレイ時のメインゲームセッション用。
  """
  def default_room, do: @default_room

  @impl true
  def init(_opts) do
    DynamicSupervisor.init(strategy: :one_for_one)
  end
end
