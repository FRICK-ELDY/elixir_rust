defmodule Game.GameLoop do
  @moduledoc """
  60 Hz game loop implemented as a GenServer.

  G2: シーン管理システム — SceneManager がスタックでシーンを管理。
  各シーンが独立した init/update を持ち、GameLoop は tick をディスパッチする。

  Phase transitions (SceneManager + 各シーンが管理):
    :playing    → :boss_alert  (ボス出現タイミングに達したとき)
    :boss_alert → :playing     (警告演出が終わったとき → Rust に spawn_boss を指示)
    :playing    → :level_up    (レベルアップ待機フラグが立ったとき)
    :level_up   → :playing     (武器選択が完了したとき)
    :playing    → :game_over   (プレイヤー HP が 0 になったとき)
  """

  use GenServer
  require Logger

  @tick_ms 16

  # ── Public API ──────────────────────────────────────────────────

  def start_link(opts), do: GenServer.start_link(__MODULE__, opts, name: __MODULE__)

  # ── GenServer callbacks ─────────────────────────────────────────

  @impl true
  def init(_opts) do
    world_ref = Game.NifBridge.create_world()
    Game.FrameCache.init()
    start_ms = now_ms()
    Process.send_after(self(), :tick, @tick_ms)

    initial_weapon_levels = fetch_weapon_levels(world_ref)

    # SceneManager は Application で GameLoop より前に起動
    {:ok, %{
      world_ref: world_ref,
      last_tick: start_ms,
      frame_count: 0,
      start_ms: start_ms,
      last_spawn_ms: start_ms,
      weapon_levels: initial_weapon_levels,
    }}
  end

  @impl true
  def handle_cast({:select_weapon, :__skip__}, state) do
    case Game.SceneManager.current() do
      {:ok, %{module: Game.Scenes.LevelUp}} ->
        Game.NifBridge.skip_level_up(state.world_ref)
        Logger.info("[LEVEL UP] Skipped weapon selection -> resuming")
        Game.SceneManager.pop_scene()
        {:noreply, %{state | weapon_levels: fetch_weapon_levels(state.world_ref)}}

      _ ->
        {:noreply, state}
    end
  end

  @impl true
  def handle_cast({:select_weapon, weapon}, state) do
    case Game.SceneManager.current() do
      {:ok, %{module: Game.Scenes.LevelUp}} ->
        Game.NifBridge.add_weapon(state.world_ref, to_string(weapon))
        new_weapon_levels = fetch_weapon_levels(state.world_ref)
        lv = Map.get(new_weapon_levels, weapon, 1)
        Logger.info("[LEVEL UP] Weapon selected: #{Game.LevelSystem.weapon_label(weapon, lv)} -> resuming")
        Game.SceneManager.pop_scene()
        {:noreply, %{state | weapon_levels: new_weapon_levels}}

      _ ->
        {:noreply, state}
    end
  end

  # ── Tick: 全シーン共通ディスパッチ ───────────────────────────────

  @impl true
  def handle_info(:tick, state) do
    now = now_ms()
    delta = now - state.last_tick
    elapsed = now - state.start_ms

    case Game.SceneManager.current() do
      :empty ->
        Process.send_after(self(), :tick, @tick_ms)
        {:noreply, %{state | last_tick: now}}

      {:ok, %{module: mod, state: scene_state}} ->
        # Playing シーンのみ物理演算・入力・イベント処理
        state = maybe_run_physics(state, mod, delta)

        context = build_context(state, now, elapsed)
        result = mod.update(context, scene_state)

        # シーンの state を SceneManager に反映
        {new_scene_state, opts} = extract_state_and_opts(result)
        Game.SceneManager.update_current(fn _ -> new_scene_state end)

        # context_updates の適用
        state = apply_context_updates(state, opts)

        # 遷移の処理
        state = process_transition(result, state, now)

        # 毎秒ログ出力 + ETS キャッシュ
        state = maybe_log_and_cache(state, mod, elapsed)

        Process.send_after(self(), :tick, @tick_ms)

        {:noreply, %{state |
          last_tick: now,
          frame_count: state.frame_count + 1,
        }}
    end
  end

  # ── ヘルパー ────────────────────────────────────────────────────

  defp maybe_run_physics(state, Game.Scenes.Playing, delta) do
    {dx, dy} = Game.InputHandler.get_move_vector()
    Game.NifBridge.set_player_input(state.world_ref, dx * 1.0, dy * 1.0)
    _ = Game.NifBridge.physics_step(state.world_ref, delta * 1.0)
    events = Game.NifBridge.drain_frame_events(state.world_ref)
    unless events == [], do: Game.EventBus.broadcast(events)
    state
  end
  defp maybe_run_physics(state, _mod, _delta), do: state

  defp build_context(state, now, elapsed) do
    %{
      tick_ms:       @tick_ms,
      world_ref:     state.world_ref,
      now:           now,
      elapsed:       elapsed,
      last_spawn_ms: state.last_spawn_ms,
      weapon_levels: state.weapon_levels,
      frame_count:   state.frame_count,
      start_ms:      state.start_ms,
    }
  end

  defp extract_state_and_opts({:continue, scene_state}), do: {scene_state, %{}}
  defp extract_state_and_opts({:continue, scene_state, opts}), do: {scene_state, opts || %{}}
  defp extract_state_and_opts({:transition, _action, scene_state}), do: {scene_state, %{}}
  defp extract_state_and_opts({:transition, _action, scene_state, opts}), do: {scene_state, opts || %{}}

  defp apply_context_updates(state, %{context_updates: updates}) when is_map(updates) do
    Map.merge(state, updates)
  end
  defp apply_context_updates(state, _), do: state

  defp process_transition({:continue, _, _}, state, _now), do: state

  defp process_transition({:transition, :pop, scene_state}, state, _now) do
    auto_select = Map.get(scene_state, :auto_select, false)
    if auto_select do
      # ポップ前に武器選択を適用（pop 後は LevelUp でなくなるため）
      state =
        case scene_state do
          %{choices: [first | _]} ->
            Game.NifBridge.add_weapon(state.world_ref, to_string(first))
            new_levels = fetch_weapon_levels(state.world_ref)
            Logger.info("[LEVEL UP] Auto-selected: #{Game.LevelSystem.weapon_label(first, Map.get(new_levels, first, 1))} -> resuming")
            %{state | weapon_levels: new_levels}
          _ ->
            Game.NifBridge.skip_level_up(state.world_ref)
            Logger.info("[LEVEL UP] Auto-skipped (no choices) -> resuming")
            state
        end
      Game.SceneManager.pop_scene()
    else
      Game.SceneManager.pop_scene()
    end
    state
  end

  defp process_transition({:transition, {:push, Game.Scenes.BossAlert, init_arg}, _}, state, _now) do
    Game.SceneManager.push_scene(Game.Scenes.BossAlert, init_arg)
    state
  end

  defp process_transition({:transition, {:push, mod, init_arg}, _}, state, _now) do
    Game.SceneManager.push_scene(mod, init_arg)
    state
  end

  defp process_transition({:transition, {:replace, Game.Scenes.GameOver, _}, _}, state, now) do
    {{_hp, _max_hp, score, _elapsed}, _counts, _level_up, _boss} =
      Game.NifBridge.get_frame_metadata(state.world_ref)
    :telemetry.execute(
      [:game, :session_end],
      %{elapsed_seconds: (now - state.start_ms) / 1000.0, score: score},
      %{}
    )
    Game.SceneManager.replace_scene(Game.Scenes.GameOver, %{})
    state
  end

  defp process_transition(_, state, _), do: state

  defp maybe_log_and_cache(state, _mod, _elapsed) do
    if rem(state.frame_count, 60) == 0 do
      # Q2: 1回のNIFで全メタデータ取得（オーバーヘッド対策）
      {{hp, max_hp, _score, elapsed_s}, {enemy_count, bullet_count, physics_ms},
       {exp, level, _level_up_pending, _exp_to_next}, {boss_alive, boss_hp, boss_max_hp}} =
        Game.NifBridge.get_frame_metadata(state.world_ref)

      hud_data = {hp, max_hp, _score, elapsed_s}
      render_type = Game.SceneManager.render_type()
      Game.FrameCache.put(enemy_count, bullet_count, physics_ms, hud_data, render_type)

      wave = Game.SpawnSystem.wave_label(elapsed_s)
      budget_warn = if physics_ms > @tick_ms, do: " [OVER BUDGET]", else: ""

      weapon_info =
        state.weapon_levels
        |> Enum.map_join(", ", fn {w, lv} -> "#{w}:Lv#{lv}" end)

      boss_info =
        if boss_alive and boss_max_hp > 0 do
          " | boss=#{Float.round(boss_hp / boss_max_hp * 100, 1)}%HP"
        else
          ""
        end

      Logger.info(
        "[LOOP] #{wave} | scene=#{render_type} | enemies=#{enemy_count} | " <>
          "physics=#{Float.round(physics_ms, 2)}ms#{budget_warn} | " <>
          "lv=#{level} exp=#{exp} | weapons=[#{weapon_info}]" <> boss_info
      )

      :telemetry.execute(
        [:game, :tick],
        %{physics_ms: physics_ms, enemy_count: enemy_count},
        %{phase: render_type, wave: wave}
      )
    end
    state
  end

  defp now_ms, do: System.monotonic_time(:millisecond)

  defp fetch_weapon_levels(world_ref) do
    world_ref
    |> Game.NifBridge.get_weapon_levels()
    |> Map.new(fn {name, level} -> {String.to_existing_atom(name), level} end)
  end
end
