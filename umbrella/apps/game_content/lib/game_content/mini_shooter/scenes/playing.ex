# Path: umbrella/apps/game_content/lib/game_content/mini_shooter/scenes/playing.ex
# Summary: MiniShooter のプレイ中シーン
defmodule GameContent.MiniShooter.Scenes.Playing do
  @behaviour GameEngine.SceneBehaviour

  require Logger

  @impl GameEngine.SceneBehaviour
  def init(_init_arg), do: {:ok, %{}}

  @impl GameEngine.SceneBehaviour
  def render_type, do: :playing

  @impl GameEngine.SceneBehaviour
  def update(context, state) do
    %{world_ref: world_ref, elapsed: elapsed, last_spawn_ms: last_spawn_ms} = context

    if GameEngine.is_player_dead?(world_ref) do
      Logger.info("[GAME OVER] Player HP reached 0 at #{div(elapsed, 1000)}s")
      {:transition, {:replace, GameContent.MiniShooter.Scenes.GameOver, %{}}, state}
    else
      new_last_spawn =
        GameContent.MiniShooter.SpawnSystem.maybe_spawn(world_ref, elapsed, last_spawn_ms)
      {:continue, state, %{context_updates: %{last_spawn_ms: new_last_spawn}}}
    end
  end
end
