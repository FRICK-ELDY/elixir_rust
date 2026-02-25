# Path: lib/engine/game_events.ex
# Summary: フレームイベント受信・フェーズ管理・NIF 呼び出しの GenServer
defmodule Engine.GameEvents do
  @moduledoc """
  Rust からの frame_events を受信し、フェーズ管理・NIF 呼び出しを行う GenServer。

  1.5.1: tick 駆動は Rust 側で高精度 60 Hz。Elixir は `{:frame_events, events}` を
  受信してイベント駆動でシーン制御・入力設定・EventBus 配信を行う。

  1.5.4: ルーム単位で複数インスタンスが起動可能。
  :main ルームのみが SceneManager・FrameCache を駆動する（表示・入力対象）。
  その他のルームは headless で physics のみ実行（Phoenix マルチプレイの土台）。

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

  def start_link(opts \\ []) do
    room_id = Keyword.get(opts, :room_id, :main)
    name = process_name(room_id)
    GenServer.start_link(__MODULE__, opts, name: name)
  end

  defp process_name(:main), do: __MODULE__
  defp process_name(room_id), do: {:via, Registry, {Engine.RoomRegistry, room_id}}

  @doc """
  1.5.3: 現在のゲーム状態をセーブする。
  """
  def save_session, do: GenServer.cast(__MODULE__, :save_session)

  @doc """
  1.5.3: セーブデータをロードしてプレイ画面に戻る。
  - `:ok` - ロード成功
  - `:no_save` - セーブファイルなし
  - `{:error, reason}` - ロード失敗
  """
  def load_session, do: GenServer.call(__MODULE__, :load_session, 5_000)

  # ── GenServer callbacks ─────────────────────────────────────────

  @impl true
  def init(opts) do
    room_id = Keyword.get(opts, :room_id, :main)

    # 1.5.4: :main は Registry に明示登録（list_rooms で列挙するため）
    if room_id == :main do
      Engine.RoomRegistry.register(:main)
    end

    world_ref = Engine.create_world()

    # 1.5.2: マップ障害物をロード
    map_id = Application.get_env(:game, :map, :plain)
    obstacles = Engine.MapLoader.obstacles_for_map(map_id)
    Engine.set_map_obstacles(world_ref, obstacles)

    control_ref = Engine.create_game_loop_control()
    if room_id == :main, do: Engine.FrameCache.init()
    start_ms = now_ms()

    # 1.5.1: Rust 駆動ゲームループを起動（高精度 60Hz）。pid は当 GenServer（GameEvents）の self()
    Engine.start_rust_game_loop(world_ref, control_ref, self())

    # 1.7.6: main ルームのみ描画スレッドを起動
    if room_id == :main, do: Engine.start_render_thread(world_ref)

    initial_weapon_levels = fetch_weapon_levels(world_ref)

    {:ok, %{
      room_id: room_id,
      world_ref: world_ref,
      control_ref: control_ref,
      last_tick: start_ms,
      frame_count: 0,
      start_ms: start_ms,
      last_spawn_ms: start_ms,
      weapon_levels: initial_weapon_levels,
    }}
  end

  @impl true
  def terminate(_reason, %{room_id: :main}) do
    Engine.RoomRegistry.unregister(:main)
    :ok
  end
  def terminate(_reason, _state), do: :ok

  @impl true
  def handle_cast({:select_weapon, :__skip__}, state) do
    game = Application.get_env(:game, :current, Game.VampireSurvivor)
    level_up_scene = game.level_up_scene()

    case Engine.SceneManager.current() do
      {:ok, %{module: ^level_up_scene}} ->
        Engine.skip_level_up(state.world_ref)
        Logger.info("[LEVEL UP] Skipped weapon selection -> resuming")
        Engine.resume_physics(state.control_ref)
        Engine.SceneManager.pop_scene()
        {:noreply, %{state | weapon_levels: fetch_weapon_levels(state.world_ref)}}

      _ ->
        {:noreply, state}
    end
  end

  @impl true
  def handle_cast(:save_session, state) do
    case Engine.save_session(state.world_ref) do
      :ok -> Logger.info("[SAVE] Session saved")
      {:error, reason} -> Logger.warning("[SAVE] Failed: #{inspect(reason)}")
    end
    {:noreply, state}
  end

  @impl true
  def handle_call(:load_session, _from, state) do
    game = Application.get_env(:game, :current, Game.VampireSurvivor)
    result = Engine.load_session(state.world_ref)

    case result do
      :ok ->
        Engine.SceneManager.replace_scene(game.physics_scenes() |> List.first(), %{})
        {:reply, :ok, %{state | weapon_levels: fetch_weapon_levels(state.world_ref)}}

      other ->
        {:reply, other, state}
    end
  end

  @impl true
  def handle_cast({:select_weapon, weapon}, state) do
    game = Application.get_env(:game, :current, Game.VampireSurvivor)
    level_up_scene = game.level_up_scene()

    case Engine.SceneManager.current() do
      {:ok, %{module: ^level_up_scene}} ->
        Engine.add_weapon(state.world_ref, weapon)
        new_weapon_levels = fetch_weapon_levels(state.world_ref)
        lv = Map.get(new_weapon_levels, weapon, 1)
        Logger.info("[LEVEL UP] Weapon selected: #{game.weapon_label(weapon, lv)} -> resuming")
        Engine.resume_physics(state.control_ref)
        Engine.SceneManager.pop_scene()
        {:noreply, %{state | weapon_levels: new_weapon_levels}}

      _ ->
        {:noreply, state}
    end
  end

  # ── 1.7.5: 描画スレッドからの UI アクションを受信 ────────────────

  @impl true
  def handle_info({:ui_action, action}, state) when is_binary(action) do
    new_state =
      case action do
        "__skip__" ->
          handle_ui_action_skip(state)

        "__save__" ->
          GenServer.cast(self(), :save_session)
          state

        "__load__" ->
          handle_ui_action_load(state)

        "__load_confirm__" ->
          handle_ui_action_load_confirm(state)

        "__load_cancel__" ->
          state

        "__start__" ->
          state

        "__retry__" ->
          state

        weapon when is_binary(weapon) ->
          handle_ui_action_weapon(state, weapon)
      end
    {:noreply, new_state}
  end

  defp handle_ui_action_skip(state) do
    {_exp, _level, level_up_pending, _exp_to_next} = Engine.get_level_up_data(state.world_ref)

    if level_up_pending do
      Engine.skip_level_up(state.world_ref)
      Logger.info("[LEVEL UP] Skipped from renderer UI")
    end

    state
    |> maybe_close_level_up_scene()
    |> Map.put(:weapon_levels, fetch_weapon_levels(state.world_ref))
  end

  defp handle_ui_action_weapon(state, weapon) do
    {_exp, _level, level_up_pending, _exp_to_next} = Engine.get_level_up_data(state.world_ref)

    if level_up_pending do
      {selected_weapon, new_levels} = apply_weapon_selection(state, weapon)
      if selected_weapon != :__skip__ do
        level = Map.get(new_levels, selected_weapon, 1)
        game = Application.get_env(:game, :current, Game.VampireSurvivor)
        Logger.info("[LEVEL UP] Weapon selected from renderer: #{game.weapon_label(selected_weapon, level)}")
      end

      %{state | weapon_levels: new_levels}
      |> maybe_close_level_up_scene()
    else
      state
    end
  end

  defp apply_weapon_selection(state, weapon_name) when is_binary(weapon_name) do
    requested_weapon =
      try do
        String.to_existing_atom(weapon_name)
      rescue
        ArgumentError -> nil
      end

    game = Application.get_env(:game, :current, Game.VampireSurvivor)
    allowed_weapons = game.entity_registry().weapons |> Map.keys() |> MapSet.new()
    fallback_weapon = Map.keys(state.weapon_levels) |> List.first() || :magic_wand

    cond do
      is_atom(requested_weapon) and MapSet.member?(allowed_weapons, requested_weapon) ->
        Engine.add_weapon(state.world_ref, requested_weapon)
        {requested_weapon, fetch_weapon_levels(state.world_ref)}

      MapSet.member?(allowed_weapons, fallback_weapon) ->
        Logger.warning(
          "[LEVEL UP] Renderer weapon '#{weapon_name}' is not available for current game. " <>
            "Falling back to #{inspect(fallback_weapon)}."
        )
        Engine.add_weapon(state.world_ref, fallback_weapon)
        {fallback_weapon, fetch_weapon_levels(state.world_ref)}

      true ->
        Logger.warning(
          "[LEVEL UP] Renderer weapon '#{weapon_name}' is not available and no valid fallback found. " <>
            "Skipping level-up."
        )
        Engine.skip_level_up(state.world_ref)
        {:__skip__, fetch_weapon_levels(state.world_ref)}
    end
  end

  defp handle_ui_action_load(state) do
    if Engine.has_save?() do
      do_load_session(state)
    else
      Logger.info("[LOAD] No save file")
      state
    end
  end

  defp handle_ui_action_load_confirm(state) do
    do_load_session(state)
  end

  defp do_load_session(state) do
    case Engine.load_session(state.world_ref) do
      :ok ->
        game = Application.get_env(:game, :current, Game.VampireSurvivor)
        Engine.SceneManager.replace_scene(game.physics_scenes() |> List.first(), %{})
        %{state | weapon_levels: fetch_weapon_levels(state.world_ref)}

      :no_save ->
        Logger.info("[LOAD] No save data")
        state

      {:error, reason} ->
        Logger.warning("[LOAD] Failed: #{inspect(reason)}")
        state
    end
  end

  defp maybe_close_level_up_scene(state) do
    game = Application.get_env(:game, :current, Game.VampireSurvivor)
    level_up_scene = game.level_up_scene()

    case Engine.SceneManager.current() do
      {:ok, %{module: ^level_up_scene}} ->
        Engine.resume_physics(state.control_ref)
        Engine.SceneManager.pop_scene()
        state

      _ ->
        state
    end
  end

  # ── 1.5.1: Rust からの frame_events を受信してシーン更新 ─────────

  @impl true
  def handle_info({:frame_events, events}, state) do
    # 1.5.4: 非 main ルームは headless（physics のみ、SceneManager 非使用）
    if state.room_id != :main do
      {:noreply, %{state | last_tick: now_ms(), frame_count: state.frame_count + 1}}
    else
      handle_frame_events_main(events, state)
    end
  end

  defp handle_frame_events_main(events, state) do
    now = now_ms()
    elapsed = now - state.start_ms

    game = Application.get_env(:game, :current, Game.VampireSurvivor)
    physics_scenes = game.physics_scenes()

    case Engine.SceneManager.current() do
      :empty ->
        {:noreply, %{state | last_tick: now}}

      {:ok, %{module: mod, state: scene_state}} ->
        state = maybe_set_input_and_broadcast(state, mod, physics_scenes, events)

        context = build_context(state, now, elapsed)
        result = mod.update(context, scene_state)

        {new_scene_state, opts} = extract_state_and_opts(result)
        Engine.SceneManager.update_current(fn _ -> new_scene_state end)

        state = apply_context_updates(state, opts)
        state = process_transition(result, state, now, game)
        state = maybe_log_and_cache(state, mod, elapsed, game)

        {:noreply, %{state |
          last_tick: now,
          frame_count: state.frame_count + 1,
        }}
    end
  end

  # ── ヘルパー ────────────────────────────────────────────────────

  # 1.5.1: Rust が physics を実行済み。入力設定とイベント配信のみ。
  # 1.7.8: :main は winit（Rust 側）で入力を直接 world に反映するため、
  # ここで ETS 入力（InputHandler）を上書きしない。
  defp maybe_set_input_and_broadcast(state, mod, physics_scenes, events) do
    if mod in physics_scenes do
      if state.room_id != :main do
        {dx, dy} = Engine.InputHandler.get_move_vector()
        Engine.set_player_input(state.world_ref, dx * 1.0, dy * 1.0)
      end
      unless events == [], do: Engine.EventBus.broadcast(events)
    end

    state
  end

  defp build_context(state, now, elapsed) do
    base = %{
      tick_ms:       @tick_ms,
      world_ref:     state.world_ref,
      now:           now,
      elapsed:       elapsed,
      last_spawn_ms: state.last_spawn_ms,
      weapon_levels: state.weapon_levels,
      frame_count:   state.frame_count,
      start_ms:      state.start_ms,
    }
    game = Application.get_env(:game, :current, Game.VampireSurvivor)
    Map.merge(game.context_defaults(), base)
  end

  defp extract_state_and_opts({:continue, scene_state}), do: {scene_state, %{}}
  defp extract_state_and_opts({:continue, scene_state, opts}), do: {scene_state, opts || %{}}
  defp extract_state_and_opts({:transition, _action, scene_state}), do: {scene_state, %{}}
  defp extract_state_and_opts({:transition, _action, scene_state, opts}), do: {scene_state, opts || %{}}

  defp apply_context_updates(state, %{context_updates: updates}) when is_map(updates) do
    Map.merge(state, updates)
  end
  defp apply_context_updates(state, _), do: state

  defp process_transition({:continue, _, _}, state, _now, _game), do: state

  defp process_transition({:transition, :pop, scene_state}, state, _now, game) do
    auto_select = Map.get(scene_state, :auto_select, false)

    if auto_select do
      state =
        case scene_state do
          %{choices: [first | _]} ->
            Engine.add_weapon(state.world_ref, first)
            new_levels = fetch_weapon_levels(state.world_ref)
            Logger.info("[LEVEL UP] Auto-selected: #{game.weapon_label(first, Map.get(new_levels, first, 1))} -> resuming")
            %{state | weapon_levels: new_levels}
          _ ->
            Engine.skip_level_up(state.world_ref)
            Logger.info("[LEVEL UP] Auto-skipped (no choices) -> resuming")
            state
        end
      Engine.resume_physics(state.control_ref)
      Engine.SceneManager.pop_scene()
    else
      Engine.resume_physics(state.control_ref)
      Engine.SceneManager.pop_scene()
    end
    state
  end

  defp process_transition({:transition, {:push, mod, init_arg}, _}, state, _now, game) do
    # LevelUp・BossAlert 中は physics を停止
    if mod == game.level_up_scene() or mod == game.boss_alert_scene() do
      Engine.pause_physics(state.control_ref)
    end

    Engine.SceneManager.push_scene(mod, init_arg)
    state
  end

  defp process_transition({:transition, {:replace, mod, init_arg}, _}, state, now, game) do
    game_over_scene = game.game_over_scene()

    init_arg =
      if mod == game_over_scene do
        {{_hp, _max_hp, score, _elapsed}, _counts, _level_up, _boss} =
          App.NifBridge.get_frame_metadata(state.world_ref)

        :telemetry.execute(
          [:game, :session_end],
          %{elapsed_seconds: (now - state.start_ms) / 1000.0, score: score},
          %{}
        )

        # 1.5.3: ハイスコアを保存
        Engine.save_high_score(score)

        Map.merge(init_arg || %{}, %{high_scores: Engine.load_high_scores()})
      else
        init_arg || %{}
      end

    Engine.SceneManager.replace_scene(mod, init_arg)
    state
  end

  defp process_transition(_, state, _, _), do: state

  defp maybe_log_and_cache(state, _mod, _elapsed, game) do
    # 1.5.4: FrameCache は main ルームのみ更新
    if state.room_id == :main and rem(state.frame_count, 60) == 0 do
      {{hp, max_hp, score, elapsed_s}, {enemy_count, bullet_count, physics_ms},
       {exp, level, _level_up_pending, _exp_to_next}, {boss_alive, boss_hp, boss_max_hp}} =
        App.NifBridge.get_frame_metadata(state.world_ref)

      hud_data = {hp, max_hp, score, elapsed_s}
      render_type = Engine.SceneManager.render_type()
      high_scores = if render_type == :game_over, do: Engine.load_high_scores(), else: nil
      Engine.FrameCache.put(enemy_count, bullet_count, physics_ms, hud_data, render_type, high_scores)

      wave = game.wave_label(elapsed_s)
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
    |> App.NifBridge.get_weapon_levels()
    |> Map.new(fn {name, level} -> {String.to_existing_atom(name), level} end)
  end
end
