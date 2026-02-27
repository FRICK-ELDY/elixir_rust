# Path: umbrella/apps/game_engine/lib/game_engine/application.ex
# Summary: OTP Application 起動（tick_hz・headless 対応・子プロセス管理）
defmodule GameEngine.Application do
  use Application

  @impl true
  def start(_type, _args) do
    headless = Application.get_env(:game_engine, :headless, false)

    game = Application.get_env(:game_engine, :current)
    if game do
      assets_path = if function_exported?(game, :assets_path, 0), do: game.assets_path(), else: ""
      System.put_env("GAME_ASSETS_ID", assets_path)
    end

    children = build_children(headless, nil)
    opts = [strategy: :one_for_one, name: GameEngine.Supervisor]
    result = Supervisor.start_link(children, opts)

    # :main ルームを起動（Rust NIF が必要なため Application 起動後に実行）
    if result != {:error, :already_started} do
      Task.start(fn ->
        Process.sleep(100)
        case GameEngine.RoomSupervisor.start_room(:main) do
          {:ok, _} -> :ok
          {:error, :already_started} -> :ok
          {:error, reason} ->
            require Logger
            Logger.warning("[GameEngine] Failed to start main room: #{inspect(reason)}")
        end
      end)
    end

    result
  end

  defp build_children(_headless, _tick_interval_ms) do
    # GameEngine.NifBridge は use Rustler により自動ロードされる（GenServer ではない）
    [
      {Registry, [keys: :unique, name: GameEngine.RoomRegistry]},
      GameEngine.SceneManager,
      GameEngine.InputHandler,
      GameEngine.EventBus,
      GameEngine.RoomSupervisor,
      GameEngine.StressMonitor,
      GameEngine.Stats,
      GameEngine.Telemetry
    ]
  end
end
