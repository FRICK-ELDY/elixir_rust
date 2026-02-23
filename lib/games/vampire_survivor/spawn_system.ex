defmodule Game.VampireSurvivor.SpawnSystem do
  @moduledoc """
  Wave-based enemy spawn system（ヴァンサバ固有）。

  Elixir handles the entire spawn schedule as pure data transformations:
  - Wave definitions are plain maps — no mutable state
  - `maybe_spawn/3` is a pure function: same inputs always produce same outputs
  """

  @max_enemies 300

  @waves [
    {  0, 3000,   3},
    { 30, 2000,   5},
    { 60, 1500,   8},
    {120, 1000,  12},
    {180,  800,  15},
  ]

  @elite_start_sec 600
  @elite_hp_multiplier 3.0

  def maybe_spawn(world_ref, elapsed_ms, last_spawn_ms) do
    elapsed_sec = elapsed_ms / 1000.0
    {interval_ms, count} = current_wave(elapsed_sec)

    if elapsed_ms - last_spawn_ms >= interval_ms do
      current = Engine.get_enemy_count(world_ref)

      if current < @max_enemies do
        to_spawn = min(count, @max_enemies - current)
        kind = enemy_kind_for_wave(elapsed_sec)

        if elapsed_sec >= @elite_start_sec do
          spawn_with_elites(world_ref, kind, to_spawn)
        else
          Engine.spawn_enemies(world_ref, kind, to_spawn)
        end
      end

      elapsed_ms
    else
      last_spawn_ms
    end
  end

  def spawn_with_elites(world_ref, kind, count) do
    elite_count  = max(1, div(count * 3, 10))
    normal_count = count - elite_count

    if normal_count > 0 do
      Engine.spawn_enemies(world_ref, kind, normal_count)
    end

    if elite_count > 0 do
      Engine.spawn_elite_enemy(world_ref, kind, elite_count, @elite_hp_multiplier)
    end
  end

  def current_wave(elapsed_sec) do
    @waves
    |> Enum.filter(fn {start, _i, _c} -> elapsed_sec >= start end)
    |> List.last({0, 800, 20})
    |> then(fn {_start, interval, count} -> {interval, count} end)
  end

  def enemy_kind_for_wave(elapsed_sec) do
    cond do
      elapsed_sec < 30  -> :slime
      elapsed_sec < 60  -> Enum.random([:slime, :bat])
      true              -> Enum.random([:slime, :bat, :golem])
    end
  end

  def wave_label(elapsed_sec) do
    cond do
      elapsed_sec <  30  -> "Wave 1 - Tutorial"
      elapsed_sec <  60  -> "Wave 2 - Warming Up (Bat added)"
      elapsed_sec < 120  -> "Wave 3 - Getting Serious (Golem added)"
      elapsed_sec < 180  -> "Wave 4 - Intense"
      elapsed_sec < 600  -> "Wave 5 - Max"
      true               -> "Wave 6 - ELITE (HP x3)"
    end
  end
end
