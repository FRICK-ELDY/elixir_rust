defmodule Engine.StressMonitor do
  @moduledoc """
  Independent performance monitoring process.

  This demonstrates a key Elixir/OTP strength: lightweight processes.
  The StressMonitor runs completely independently from the GameLoop —
  if it crashes, the game continues unaffected (one_for_one supervision).

  It samples the game world every second and:
  - Logs a performance dashboard to the console
  - Detects frame budget overruns and emits warnings
  - Tracks peak enemy count and peak physics time
  - Automatically escalates log level when performance degrades

  All state is immutable maps; each tick produces a new state value.
  """

  use GenServer
  require Logger

  @sample_interval_ms 1_000
  @frame_budget_ms    1000.0 / 60.0
  @max_enemies        10_000

  # ── Public API ──────────────────────────────────────────────────

  def start_link(opts), do: GenServer.start_link(__MODULE__, opts, name: __MODULE__)

  def get_stats, do: GenServer.call(__MODULE__, :get_stats)

  # ── GenServer callbacks ─────────────────────────────────────────

  @impl true
  def init(_opts) do
    Process.send_after(self(), :sample, @sample_interval_ms)

    {:ok, %{
      samples:          0,
      peak_enemies:     0,
      peak_physics_ms:  0.0,
      overrun_count:    0,
      last_enemy_count: 0,
    }}
  end

  @impl true
  def handle_call(:get_stats, _from, state), do: {:reply, state, state}

  @impl true
  def handle_info(:sample, state) do
    Process.send_after(self(), :sample, @sample_interval_ms)

    new_state = sample_and_log(state)
    {:noreply, new_state}
  end

  # ── Private ─────────────────────────────────────────────────────

  defp sample_and_log(state) do
    case Engine.FrameCache.get() do
      :empty ->
        state

      {:ok, %{
        enemy_count:  enemy_count,
        bullet_count: bullet_count,
        physics_ms:   physics_ms,
        hud_data:     {hp, max_hp, score, elapsed_s},
      }} ->
        game_module = Application.get_env(:game, :current, Game.VampireSurvivor)
        wave = game_module.wave_label(elapsed_s)

        overrun = physics_ms > @frame_budget_ms

        new_state = %{state |
          samples:          state.samples + 1,
          peak_enemies:     Kernel.max(state.peak_enemies, enemy_count),
          peak_physics_ms:  Float.round(Kernel.max(state.peak_physics_ms, physics_ms), 2),
          overrun_count:    state.overrun_count + if(overrun, do: 1, else: 0),
          last_enemy_count: enemy_count,
        }

        hp_pct = if max_hp > 0, do: Float.round(hp / max_hp * 100, 1), else: 0.0
        physics_bar = perf_bar(physics_ms, @frame_budget_ms, 20)

        log_fn = if overrun, do: &Logger.warning/1, else: &Logger.info/1

        log_fn.(
          """
          \n╔══════════════════════════════════════════════════════╗
          ║  STRESS MONITOR  [#{wave}]
          ╠══════════════════════════════════════════════════════╣
          ║  Enemies  : #{pad(enemy_count, 6)} / #{@max_enemies}   (peak: #{new_state.peak_enemies})
          ║  Bullets  : #{pad(bullet_count, 6)}
          ║  Score    : #{pad(score, 6)}   HP: #{hp_pct}%
          ║  Physics  : #{Float.round(physics_ms, 2)}ms  #{physics_bar}  budget: #{@frame_budget_ms}ms
          ║  Overruns : #{new_state.overrun_count} / #{new_state.samples} samples
          ║  Peak ms  : #{new_state.peak_physics_ms}ms
          ╚══════════════════════════════════════════════════════╝\
          """
        )

        new_state
    end
  end

  defp perf_bar(value, max, width) do
    filled = round(min(value / max, 1.0) * width)
    empty  = width - filled
    color  = cond do
      value > max        -> "!!"
      value > max * 0.75 -> ">>"
      true               -> "  "
    end
    "[" <> String.duplicate("#", filled) <> String.duplicate("-", empty) <> "]" <> color
  end

  defp pad(n, width) do
    str = to_string(n)
    String.pad_leading(str, width)
  end
end
