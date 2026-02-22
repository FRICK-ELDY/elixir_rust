defmodule Game.SpawnSystem do
  @moduledoc """
  一定間隔で敵をスポーンさせるシステム。
  Step 12: 10000体到達を目標に、スポーン間隔・スポーン数を調整。
  """

  # 500ms ごとに 100 体スポーン → 約 50 秒で 10000 体に到達
  @spawn_interval_ms 500
  @spawn_count 100
  @max_enemies 10000

  @doc """
  経過時間に応じて敵をスポーンする。
  上限 `@max_enemies` 体を超えた場合はスポーンしない。
  スポーンした場合は新しい `last_spawn_ms` を、しない場合は元の値を返す。
  """
  def maybe_spawn(world_ref, elapsed_ms, last_spawn_ms) do
    if elapsed_ms - last_spawn_ms >= @spawn_interval_ms do
      current = Game.NifBridge.get_enemy_count(world_ref)
      if current < @max_enemies do
        count = min(@spawn_count, @max_enemies - current)
        Game.NifBridge.spawn_enemies(world_ref, :slime, count)
      end
      elapsed_ms
    else
      last_spawn_ms
    end
  end
end
