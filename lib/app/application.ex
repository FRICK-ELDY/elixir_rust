defmodule App.Application do
  use Application

  @impl true
  def start(_type, _args) do
    # Step 39: ゲーム別アセットパス — game_window 等が GAME_ASSETS_ID を参照
    # 常に設定することで、再起動時に前回の値が残らないようにする（空文字列は Rust 側で None として扱う）
    game = Application.get_env(:game, :current, Game.VampireSurvivor)
    assets_path = if function_exported?(game, :assets_path, 0), do: game.assets_path(), else: ""
    System.put_env("GAME_ASSETS_ID", assets_path)

    children = [
      # Step 44: ルーム ID → GameEvents pid の Registry
      {Registry, [keys: :unique, name: Engine.RoomRegistry]},
      # G2: シーン管理 — GameEvents より前に起動
      Engine.SceneManager,
      # Input handler: translates key events to GameEvents (ETS 経由)
      Engine.InputHandler,
      # Step 26: イベントバス — GameEvents より前に起動
      Engine.EventBus,
      # Step 44: ルーム管理（内部で GameEvents を起動）
      Engine.RoomSupervisor,
      # Independent performance monitor
      Engine.StressMonitor,
      # Step 25: ゲームセッション統計収集
      Engine.Stats,
      # P7: Telemetry 計測基盤
      Engine.Telemetry,
    ]

    opts = [strategy: :one_for_one, name: App.Supervisor]
    result = Supervisor.start_link(children, opts)

    # Step 44: デフォルトルーム（:main）を起動
    if elem(result, 0) == :ok do
      case Engine.RoomSupervisor.start_room(:main) do
        {:ok, _} -> :ok
        {:error, :already_started} -> :ok
        {:error, reason} -> raise "Failed to start main room: #{inspect(reason)}"
      end
    end

    result
  end
end
