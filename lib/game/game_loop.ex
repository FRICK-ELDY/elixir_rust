defmodule Game.GameLoop do
  @moduledoc """
  60 Hz game loop implemented as a GenServer.

  Elixir strengths on display:
  - The tick loop uses `Process.send_after/3` — no OS thread, just a message
  - Pattern matching on `phase` dispatches to different tick handlers cleanly
  - Immutable state: every tick returns a new map; no mutation anywhere
  - The loop is supervised: if it crashes, the supervisor restarts it instantly
  """

  use GenServer
  require Logger

  @tick_ms 16
  # Elixir-side fallback: auto-selects after this duration when no UI is connected.
  # The standalone Rust window uses button/keyboard selection instead.
  @level_up_auto_select_ms 3_000

  # ── Public API ──────────────────────────────────────────────────

  def start_link(opts), do: GenServer.start_link(__MODULE__, opts, name: __MODULE__)

  # ── GenServer callbacks ─────────────────────────────────────────

  @impl true
  def init(_opts) do
    world_ref = Game.NifBridge.create_world()
    start_ms  = now_ms()
    Process.send_after(self(), :tick, @tick_ms)

    # Initial weapon state is derived from Rust (single source of truth).
    # This way, changing the starting loadout in create_world() is enough.
    initial_weapon_levels = fetch_weapon_levels(world_ref)

    {:ok, %{
      world_ref:           world_ref,
      last_tick:           start_ms,
      frame_count:         0,
      start_ms:            start_ms,
      last_spawn_ms:       start_ms,
      phase:               :playing,
      weapon_levels:       initial_weapon_levels,
      level_up_entered_ms: nil,
      weapon_choices:      [],
    }}
  end

  # StressMonitor から world_ref を取得するためのコールバック
  @impl true
  def handle_call(:get_world_ref, _from, state) do
    {:reply, state.world_ref, state}
  end

  @impl true
  def handle_cast({:input, :move, {dx, dy}}, state) do
    Game.NifBridge.set_player_input(state.world_ref, dx * 1.0, dy * 1.0)
    {:noreply, state}
  end

  @impl true
  def handle_cast({:select_weapon, :__skip__}, %{phase: :level_up} = state) do
    Game.NifBridge.skip_level_up(state.world_ref)
    Logger.info("[LEVEL UP] Skipped weapon selection -> resuming")
    {:noreply, %{state |
      phase:               :playing,
      level_up_entered_ms: nil,
      weapon_choices:      [],
    }}
  end

  @impl true
  def handle_cast({:select_weapon, weapon}, %{phase: :level_up} = state) do
    Game.NifBridge.add_weapon(state.world_ref, to_string(weapon))

    # Sync weapon levels from Rust (single source of truth)
    new_weapon_levels = fetch_weapon_levels(state.world_ref)
    lv = Map.get(new_weapon_levels, weapon, 1)
    Logger.info("[LEVEL UP] Weapon selected: #{Game.LevelSystem.weapon_label(weapon, lv)} -> resuming")

    {:noreply, %{state |
      phase:               :playing,
      weapon_levels:       new_weapon_levels,
      level_up_entered_ms: nil,
      weapon_choices:      [],
    }}
  end

  def handle_cast({:select_weapon, _weapon}, state), do: {:noreply, state}

  # ── Tick: level_up phase (physics paused) ───────────────────────

  @impl true
  def handle_info(:tick, %{phase: :level_up} = state) do
    now = now_ms()
    Process.send_after(self(), :tick, @tick_ms)

    elapsed_in_level_up = now - (state.level_up_entered_ms || now)
    if elapsed_in_level_up >= @level_up_auto_select_ms do
      chosen = List.first(state.weapon_choices) || :__skip__
      GenServer.cast(self(), {:select_weapon, chosen})
    end

    {:noreply, %{state | last_tick: now}}
  end

  # ── Tick: playing phase ─────────────────────────────────────────

  @impl true
  def handle_info(:tick, state) do
    now     = now_ms()
    delta   = now - state.last_tick
    elapsed = now - state.start_ms

    # Physics step runs in Rust (DirtyCpu NIF — won't block the BEAM scheduler)
    _frame_id = Game.NifBridge.physics_step(state.world_ref, delta * 1.0)

    # Spawn system: pure Elixir wave logic decides when/how many to spawn
    new_last_spawn =
      Game.SpawnSystem.maybe_spawn(state.world_ref, elapsed, state.last_spawn_ms)

    # Level-up check
    {exp, level, level_up_pending, exp_to_next} =
      Game.NifBridge.get_level_up_data(state.world_ref)

    new_state =
      if level_up_pending and state.phase == :playing do
        # Step 17: weapon_levels マップを使って選択肢を生成
        choices = Game.LevelSystem.generate_weapon_choices(state.weapon_levels)

        if choices == [] do
          # 全武器がMaxLvの場合は選択肢なし → 即スキップしてゲーム再開
          Logger.info("[LEVEL UP] All weapons at max level — skipping weapon selection")
          Game.NifBridge.skip_level_up(state.world_ref)
          state
        else
          choice_labels =
            Enum.map_join(choices, " / ", fn w ->
              lv = Map.get(state.weapon_levels, w, 0)
              Game.LevelSystem.weapon_label(w, lv)
            end)

          Logger.info(
            "[LEVEL UP] Level #{level} -> #{level + 1} | " <>
            "EXP: #{exp} | to next: #{exp_to_next} | " <>
            "choices: #{choice_labels}"
          )
          Logger.info("[LEVEL UP] Waiting for player selection...")

          %{state |
            phase:               :level_up,
            weapon_choices:      choices,
            level_up_entered_ms: now,
          }
        end
      else
        state
      end

    # Log a compact status line every second (60 frames)
    if rem(state.frame_count, 60) == 0 do
      enemy_count   = Game.NifBridge.get_enemy_count(state.world_ref)
      physics_ms    = Game.NifBridge.get_frame_time_ms(state.world_ref)
      {_hp, _max_hp, _score, elapsed_s} = Game.NifBridge.get_hud_data(state.world_ref)
      wave          = Game.SpawnSystem.wave_label(elapsed_s)
      budget_warn   = if physics_ms > @tick_ms, do: " [OVER BUDGET]", else: ""

      # Step 17: 武器レベルを HUD ログに表示
      weapon_info =
        new_state.weapon_levels
        |> Enum.map_join(", ", fn {w, lv} -> "#{w}:Lv#{lv}" end)

      Logger.info(
        "[LOOP] #{wave} | enemies=#{enemy_count} | " <>
        "physics=#{Float.round(physics_ms, 2)}ms#{budget_warn} | " <>
        "lv=#{level} exp=#{exp}(+#{exp_to_next}) | weapons=[#{weapon_info}]"
      )
    end

    Process.send_after(self(), :tick, @tick_ms)

    {:noreply, %{new_state |
      last_tick:     now,
      frame_count:   state.frame_count + 1,
      last_spawn_ms: new_last_spawn,
    }}
  end

  defp now_ms, do: System.monotonic_time(:millisecond)

  # Converts [{weapon_name_string, level}] from Rust into %{weapon_atom => level}.
  # String.to_existing_atom/1 is used intentionally: it raises ArgumentError for
  # any unknown weapon name, preventing atom table exhaustion from unexpected NIF output.
  defp fetch_weapon_levels(world_ref) do
    world_ref
    |> Game.NifBridge.get_weapon_levels()
    |> Map.new(fn {name, level} -> {String.to_existing_atom(name), level} end)
  end
end
