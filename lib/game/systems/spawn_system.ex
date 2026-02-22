defmodule Game.SpawnSystem do
  @moduledoc """
  一定間隔で敵をスポーンさせるシステム。
  """

  @spawn_interval_ms 2000
  @spawn_count 10

  @doc """
  経過時間に応じて敵をスポーンする。
  スポーンした場合は新しい `last_spawn_ms` を、しない場合は元の値を返す。
  """
  def maybe_spawn(world_ref, elapsed_ms, last_spawn_ms) do
    if elapsed_ms - last_spawn_ms >= @spawn_interval_ms do
      Game.NifBridge.spawn_enemies(world_ref, :slime, @spawn_count)
      elapsed_ms
    else
      last_spawn_ms
    end
  end
end
