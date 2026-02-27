# Path: umbrella/apps/game_content/lib/game_content/mini_shooter/spawn_system.ex
# Summary: ミニマル敵スポーンシステム（スライム固定間隔）
defmodule GameContent.MiniShooter.SpawnSystem do
  @max_enemies 50
  @spawn_interval_ms 800

  def maybe_spawn(world_ref, elapsed_ms, last_spawn_ms) do
    if elapsed_ms - last_spawn_ms >= @spawn_interval_ms do
      current = GameEngine.get_enemy_count(world_ref)

      if current < @max_enemies do
        to_spawn = min(2, @max_enemies - current)
        GameEngine.spawn_enemies(world_ref, :slime, to_spawn)
      end

      elapsed_ms
    else
      last_spawn_ms
    end
  end
end
