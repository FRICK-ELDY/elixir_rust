defmodule Game.GameLoop do
  @moduledoc """
  60 Hz game loop implemented as a GenServer.

  Elixir strengths on display:
  - The tick loop uses `Process.send_after/3` — no OS thread, just a message
  - Pattern matching on `phase` dispatches to different tick handlers cleanly
  - Immutable state: every tick returns a new map; no mutation anywhere
  - The loop is supervised: if it crashes, the supervisor restarts it instantly

  Phase transitions (Elixir が司令塔として管理):
    :playing    → :boss_alert  (ボス出現タイミングに達したとき)
    :boss_alert → :playing     (警告演出が終わったとき → Rust に spawn_boss を指示)
    :playing    → :level_up    (レベルアップ待機フラグが立ったとき)
    :level_up   → :playing     (武器選択が完了したとき)
    :playing    → :game_over   (プレイヤー HP が 0 になったとき)
    :game_over  → :playing     (リスタートしたとき)
  """

  use GenServer
  require Logger

  @tick_ms 16
  # Elixir-side fallback: auto-selects after this duration when no UI is connected.
  @level_up_auto_select_ms 3_000

  # ── Public API ──────────────────────────────────────────────────

  def start_link(opts), do: GenServer.start_link(__MODULE__, opts, name: __MODULE__)

  # ── GenServer callbacks ─────────────────────────────────────────

  @impl true
  def init(_opts) do
    world_ref = Game.NifBridge.create_world()
    start_ms  = now_ms()
    Process.send_after(self(), :tick, @tick_ms)

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
      # ── ボス管理（Elixir が司令塔として保持）──────────────────
      spawned_bosses:      [],          # 既に出現したボス種別リスト
      boss_alert_ms:       nil,         # :boss_alert フェーズ開始時刻
      pending_boss:        nil,         # 警告演出後にスポーンするボス種別
      pending_boss_name:   nil,         # 警告表示用ボス名
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

  # ── Tick: game_over phase ────────────────────────────────────────

  @impl true
  def handle_info(:tick, %{phase: :game_over} = state) do
    Process.send_after(self(), :tick, @tick_ms)
    {:noreply, %{state | last_tick: now_ms()}}
  end

  # ── Tick: boss_alert phase ───────────────────────────────────────

  @impl true
  def handle_info(:tick, %{phase: :boss_alert} = state) do
    now = now_ms()
    Process.send_after(self(), :tick, @tick_ms)

    alert_elapsed = now - (state.boss_alert_ms || now)

    if alert_elapsed >= Game.BossSystem.alert_duration_ms() do
      # 警告演出終了 → Rust にボスをスポーンさせる
      Game.NifBridge.spawn_boss(state.world_ref, state.pending_boss)
      Logger.info("[BOSS] Spawned: #{state.pending_boss_name}")

      {:noreply, %{state |
        phase:             :playing,
        last_tick:         now,
        boss_alert_ms:     nil,
        pending_boss:      nil,
        pending_boss_name: nil,
      }}
    else
      {:noreply, %{state | last_tick: now}}
    end
  end

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

    # ── 1. ゲームオーバー検知（Elixir が HP 監視の司令塔）─────────
    state =
      if Game.NifBridge.is_player_dead(state.world_ref) do
        Logger.info("[GAME OVER] Player HP reached 0 at #{div(elapsed, 1000)}s")
        %{state | phase: :game_over}
      else
        state
      end

    # ゲームオーバーになった場合はここで早期リターン
    if state.phase == :game_over do
      Process.send_after(self(), :tick, @tick_ms)
      {:noreply, %{state | last_tick: now, frame_count: state.frame_count + 1}}
    else
      # ── 2. ボス出現チェック（Elixir がスケジュール管理）──────────
      elapsed_sec = elapsed / 1000.0
      {state, new_last_spawn} =
        case Game.BossSystem.check_spawn(elapsed_sec, state.spawned_bosses) do
          {:spawn, boss_kind, boss_name} ->
            Logger.info("[BOSS] Alert: #{boss_name} incoming!")
            new_state = %{state |
              phase:             :boss_alert,
              boss_alert_ms:     now,
              pending_boss:      boss_kind,
              pending_boss_name: boss_name,
              spawned_bosses:    [boss_kind | state.spawned_bosses],
            }
            {new_state, state.last_spawn_ms}

          :no_boss ->
            # ── 3. 通常スポーン（SpawnSystem が難易度エスカレーション含む）
            new_spawn_ms =
              Game.SpawnSystem.maybe_spawn(state.world_ref, elapsed, state.last_spawn_ms)
            {state, new_spawn_ms}
        end

      # ── 4. レベルアップチェック ───────────────────────────────────
      {exp, level, level_up_pending, exp_to_next} =
        Game.NifBridge.get_level_up_data(state.world_ref)

      state =
        if level_up_pending and state.phase == :playing do
          choices = Game.LevelSystem.generate_weapon_choices(state.weapon_levels)

          if choices == [] do
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

      # ── 5. 毎秒ログ出力 ──────────────────────────────────────────
      if rem(state.frame_count, 60) == 0 do
        enemy_count = Game.NifBridge.get_enemy_count(state.world_ref)
        physics_ms  = Game.NifBridge.get_frame_time_ms(state.world_ref)
        {_hp, _max_hp, _score, elapsed_s} = Game.NifBridge.get_hud_data(state.world_ref)
        wave        = Game.SpawnSystem.wave_label(elapsed_s)
        budget_warn = if physics_ms > @tick_ms, do: " [OVER BUDGET]", else: ""

        weapon_info =
          state.weapon_levels
          |> Enum.map_join(", ", fn {w, lv} -> "#{w}:Lv#{lv}" end)

        boss_info =
          case Game.NifBridge.get_boss_info(state.world_ref) do
            {:alive, hp, max_hp} ->
              " | boss=#{Float.round(hp / max_hp * 100, 1)}%HP"
            _ ->
              ""
          end

        Logger.info(
          "[LOOP] #{wave} | enemies=#{enemy_count} | " <>
          "physics=#{Float.round(physics_ms, 2)}ms#{budget_warn} | " <>
          "lv=#{level} exp=#{exp}(+#{exp_to_next}) | weapons=[#{weapon_info}]" <>
          "#{boss_info}"
        )
      end

      Process.send_after(self(), :tick, @tick_ms)

      {:noreply, %{state |
        last_tick:     now,
        frame_count:   state.frame_count + 1,
        last_spawn_ms: new_last_spawn,
      }}
    end
  end

  defp now_ms, do: System.monotonic_time(:millisecond)

  defp fetch_weapon_levels(world_ref) do
    world_ref
    |> Game.NifBridge.get_weapon_levels()
    |> Map.new(fn {name, level} -> {String.to_existing_atom(name), level} end)
  end
end
