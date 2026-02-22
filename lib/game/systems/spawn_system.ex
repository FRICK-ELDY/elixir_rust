defmodule Game.SpawnSystem do
  @moduledoc """
  Wave-based enemy spawn system.

  Elixir handles the entire spawn schedule as pure data transformations:
  - Wave definitions are plain maps — no mutable state
  - `maybe_spawn/3` is a pure function: same inputs always produce same outputs
  - The BEAM scheduler ensures this never blocks the physics tick

  Spawn curve (enemies alive target):
    0–10s   :  100 enemies  (tutorial)
    10–30s  :  500 enemies  (warming up)
    30–60s  : 1 500 enemies (getting serious)
    60–120s : 5 000 enemies (chaos)
    120s+   :10 000 enemies (maximum stress test)

  Step 18 - Enemy types by elapsed time:
    0–30s   : Slime only
    30–60s  : Slime + Bat
    60s+    : Slime + Bat + Golem
  """

  @max_enemies 10_000

  # Wave table: {start_sec, spawn_interval_ms, spawn_count_per_tick}
  @waves [
    {  0,  800,   20},
    { 10,  600,   50},
    { 30,  400,  100},
    { 60,  300,  200},
    {120,  200,  300},
  ]

  @doc """
  Spawns enemies according to the current wave schedule.
  Returns the updated `last_spawn_ms`.

  This function is intentionally a pure side-effect-free decision function:
  the actual NIF call is the only impure part, making it easy to test.
  """
  def maybe_spawn(world_ref, elapsed_ms, last_spawn_ms) do
    elapsed_sec = elapsed_ms / 1000.0
    {interval_ms, count} = current_wave(elapsed_sec)

    if elapsed_ms - last_spawn_ms >= interval_ms do
      current = Game.NifBridge.get_enemy_count(world_ref)

      if current < @max_enemies do
        to_spawn = min(count, @max_enemies - current)
        kind = enemy_kind_for_wave(elapsed_sec)
        Game.NifBridge.spawn_enemies(world_ref, kind, to_spawn)
      end

      elapsed_ms
    else
      last_spawn_ms
    end
  end

  @doc """
  Returns {interval_ms, spawn_count} for the given elapsed time in seconds.
  Uses Elixir's pattern matching to select the highest applicable wave.
  """
  def current_wave(elapsed_sec) do
    @waves
    |> Enum.filter(fn {start, _i, _c} -> elapsed_sec >= start end)
    |> List.last({0, 800, 20})
    |> then(fn {_start, interval, count} -> {interval, count} end)
  end

  @doc """
  Step 18: ウェーブ進行に応じて敵タイプを選択する。

  - 0〜30秒:  スライムのみ（チュートリアル）
  - 30〜60秒: スライム + コウモリ
  - 60秒〜:   スライム + コウモリ + ゴーレム

  純粋関数: 同じ入力に対して常に同じ出力（ランダム選択を除く）
  """
  def enemy_kind_for_wave(elapsed_sec) do
    cond do
      elapsed_sec < 30  -> :slime
      elapsed_sec < 60  -> Enum.random([:slime, :bat])
      true              -> Enum.random([:slime, :bat, :golem])
    end
  end

  @doc """
  Returns a human-readable description of the current wave phase.
  Used by StressMonitor for logging.
  """
  def wave_label(elapsed_sec) do
    cond do
      elapsed_sec <  10 -> "Wave 1 - Tutorial"
      elapsed_sec <  30 -> "Wave 2 - Warming Up"
      elapsed_sec <  60 -> "Wave 3 - Getting Serious (Bat added)"
      elapsed_sec < 120 -> "Wave 4 - Chaos (Golem added)"
      true              -> "Wave 5 - MAX STRESS"
    end
  end
end
