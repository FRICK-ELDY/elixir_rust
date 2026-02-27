# Path: umbrella/apps/game_content/lib/game_content/mini_shooter/scenes/game_over.ex
# Summary: MiniShooter のゲームオーバーシーン
defmodule GameContent.MiniShooter.Scenes.GameOver do
  @behaviour GameEngine.SceneBehaviour

  @impl GameEngine.SceneBehaviour
  def init(_init_arg), do: {:ok, %{}}

  @impl GameEngine.SceneBehaviour
  def render_type, do: :game_over

  @impl GameEngine.SceneBehaviour
  def update(_context, state), do: {:continue, state}
end
