# Path: lib/games/mini_shooter/scenes/playing.ex
# Summary: MiniShooter のプレイ中シーン
defmodule Game.MiniShooter.Scenes.Playing do
  @moduledoc """
  プレイ中シーン。敵スポーンのみ。ゲームオーバーで終了。
  """
  @behaviour Engine.SceneBehaviour

  require Logger

  @impl Engine.SceneBehaviour
  def init(_init_arg), do: {:ok, %{}}

  @impl Engine.SceneBehaviour
  def render_type, do: :playing

  @impl Engine.SceneBehaviour
  def update(context, state) do
    %{world_ref: world_ref, elapsed: elapsed, last_spawn_ms: last_spawn_ms} = context

    if Engine.is_player_dead?(world_ref) do
      Logger.info("[GAME OVER] Player HP reached 0 at #{div(elapsed, 1000)}s")
      {:transition, {:replace, Game.MiniShooter.Scenes.GameOver, %{}}, state}
    else
      new_last_spawn =
        Game.MiniShooter.SpawnSystem.maybe_spawn(world_ref, elapsed, last_spawn_ms)

      {:continue, state, %{context_updates: %{last_spawn_ms: new_last_spawn}}}
    end
  end
end
