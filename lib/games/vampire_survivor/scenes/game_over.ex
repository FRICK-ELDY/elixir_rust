# Path: lib/games/vampire_survivor/scenes/game_over.ex
# Summary: ヴァンサバのゲームオーバーシーン
defmodule Game.VampireSurvivor.Scenes.GameOver do
  @moduledoc """
  ゲームオーバーシーン。スコア表示・リトライ待機。
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
