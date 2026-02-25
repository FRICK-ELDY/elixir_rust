# Path: lib/games/mini_shooter/scenes/game_over.ex
# Summary: ミニ shooter のゲームオーバーシーン
defmodule Game.MiniShooter.Scenes.GameOver do
  @moduledoc """
  ゲームオーバーシーン。
  """
  @behaviour Engine.SceneBehaviour

  @impl Engine.SceneBehaviour
  def init(_init_arg), do: {:ok, %{}}

  @impl Engine.SceneBehaviour
  def render_type, do: :game_over

  @impl Engine.SceneBehaviour
  def update(_context, state) do
    {:continue, state}
  end
end
