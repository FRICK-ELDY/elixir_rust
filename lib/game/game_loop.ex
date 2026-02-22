defmodule Game.GameLoop do
  use GenServer
  require Logger

  @tick_ms 16

  # ── Step 14: 武器選択の自動選択ディレイ（ミリ秒）
  # 実際のゲームでは UI から選択するが、コンソール実装では自動で最初の選択肢を選ぶ
  @level_up_auto_select_ms 3000

  def start_link(opts), do: GenServer.start_link(__MODULE__, opts, name: __MODULE__)

  @impl true
  def init(_opts) do
    world_ref = Game.NifBridge.create_world()
    start_ms = now_ms()
    Process.send_after(self(), :tick, @tick_ms)

    {:ok,
     %{
       world_ref:        world_ref,
       last_tick:        start_ms,
       frame_count:      0,
       start_ms:         start_ms,
       last_spawn_ms:    start_ms,
       # ── Step 14: フェーズ管理 ──────────────────────────────
       # フェーズ: :playing | :level_up
       phase:            :playing,
       # 現在装備中の武器リスト（Elixir 側でも管理）
       weapons:          [:magic_wand],
       # レベルアップ画面に入った時刻（自動選択タイムアウト用）
       level_up_entered_ms: nil,
       # 提示中の武器選択肢
       weapon_choices:   []
     }}
  end

  @impl true
  def handle_cast({:input, :move, {dx, dy}}, state) do
    Game.NifBridge.set_player_input(state.world_ref, dx * 1.0, dy * 1.0)
    {:noreply, state}
  end

  # ── Step 14: 武器選択コマンド（外部から呼び出し可能）─────────
  # 例: GenServer.cast(Game.GameLoop, {:select_weapon, :axe})
  @impl true
  def handle_cast({:select_weapon, weapon}, %{phase: :level_up} = state) do
    chosen = to_string(weapon)
    Game.NifBridge.add_weapon(state.world_ref, chosen)
    new_weapons = Enum.uniq(state.weapons ++ [weapon])

    Logger.info("[LEVEL UP] 武器選択: #{Game.LevelSystem.weapon_label(weapon)} → ゲーム再開")

    {:noreply,
     %{state |
       phase:               :playing,
       weapons:             new_weapons,
       level_up_entered_ms: nil,
       weapon_choices:      []
     }}
  end

  def handle_cast({:select_weapon, _weapon}, state) do
    # レベルアップ中でなければ無視
    {:noreply, state}
  end

  @impl true
  def handle_info(:tick, %{phase: :level_up} = state) do
    now = now_ms()
    Process.send_after(self(), :tick, @tick_ms)

    # タイムアウト経過で自動的に最初の選択肢を選ぶ
    elapsed_in_level_up = now - (state.level_up_entered_ms || now)
    if elapsed_in_level_up >= @level_up_auto_select_ms do
      chosen = List.first(state.weapon_choices, :magic_wand)
      GenServer.cast(self(), {:select_weapon, chosen})
    end

    {:noreply, %{state | last_tick: now}}
  end

  @impl true
  def handle_info(:tick, state) do
    now     = now_ms()
    delta   = now - state.last_tick
    elapsed = now - state.start_ms

    _frame_id = Game.NifBridge.physics_step(state.world_ref, delta * 1.0)

    # 敵スポーン（2 秒ごとに 10 体）
    new_last_spawn =
      Game.SpawnSystem.maybe_spawn(state.world_ref, elapsed, state.last_spawn_ms)

    # ── Step 14: レベルアップ判定 ──────────────────────────────
    {_exp, _level, level_up_pending, _exp_to_next} =
      Game.NifBridge.get_level_up_data(state.world_ref)

    new_state =
      if level_up_pending and state.phase == :playing do
        {exp, level, _pending, exp_to_next} = Game.NifBridge.get_level_up_data(state.world_ref)
        choices = Game.LevelSystem.generate_weapon_choices(state.weapons)

        Logger.info(
          "[LEVEL UP] ★ レベルアップ！ Lv.#{level - 1} → Lv.#{level} | " <>
          "EXP: #{exp} | 次まで: #{exp_to_next} | " <>
          "選択肢: #{Enum.map_join(choices, " / ", &Game.LevelSystem.weapon_label/1)}"
        )
        Logger.info("[LEVEL UP] #{@level_up_auto_select_ms}ms 後に自動選択されます...")

        %{state |
          phase:               :level_up,
          weapon_choices:      choices,
          level_up_entered_ms: now
        }
      else
        state
      end

    if rem(state.frame_count, 60) == 0 do
      {px, py}                              = Game.NifBridge.get_player_pos(state.world_ref)
      {hp, max_hp, score, elapsed_s}        = Game.NifBridge.get_hud_data(state.world_ref)
      {exp, level, _pending, exp_to_next}   = Game.NifBridge.get_level_up_data(state.world_ref)
      enemy_count                           = Game.NifBridge.get_enemy_count(state.world_ref)
      bullet_count                          = Game.NifBridge.get_bullet_count(state.world_ref)
      frame_time_ms                         = Game.NifBridge.get_frame_time_ms(state.world_ref)
      budget_warn                           = if frame_time_ms > @tick_ms, do: " ⚠ OVER BUDGET", else: ""

      hp_bar   = hud_hp_bar(hp, max_hp)
      time_str = format_time(elapsed_s)
      exp_bar  = hud_exp_bar(exp, exp + exp_to_next)

      Logger.info(
        "[HUD] #{hp_bar} HP: #{Float.round(hp, 1)}/#{trunc(max_hp)} | " <>
        "Score: #{score} | Time: #{time_str} | " <>
        "Lv.#{level} #{exp_bar} EXP: #{exp}(+#{exp_to_next}) | " <>
        "Enemies: #{enemy_count} | Bullets: #{bullet_count} | " <>
        "Player: (#{Float.round(px, 1)}, #{Float.round(py, 1)}) | " <>
        "PhysicsTime: #{Float.round(frame_time_ms, 2)}ms#{budget_warn}"
      )
    end

    Process.send_after(self(), :tick, @tick_ms)

    {:noreply,
     %{new_state |
       last_tick:     now,
       frame_count:   state.frame_count + 1,
       last_spawn_ms: new_last_spawn
     }}
  end

  defp now_ms, do: System.monotonic_time(:millisecond)

  # ── Step 13: HUD ヘルパー ──────────────────────────────────────

  @bar_length 20

  defp hud_hp_bar(hp, max_hp) when max_hp > 0 do
    filled = round(hp / max_hp * @bar_length) |> max(0) |> min(@bar_length)
    empty  = @bar_length - filled
    "[" <> String.duplicate("#", filled) <> String.duplicate("-", empty) <> "]"
  end
  defp hud_hp_bar(_, _), do: "[" <> String.duplicate("-", @bar_length) <> "]"

  # ── Step 14: EXP バー ──────────────────────────────────────────

  @exp_bar_length 10

  defp hud_exp_bar(exp, exp_needed) when exp_needed > 0 do
    filled = round(exp / exp_needed * @exp_bar_length) |> max(0) |> min(@exp_bar_length)
    empty  = @exp_bar_length - filled
    "[" <> String.duplicate("*", filled) <> String.duplicate(".", empty) <> "]"
  end
  defp hud_exp_bar(_, _), do: "[" <> String.duplicate(".", @exp_bar_length) <> "]"

  defp format_time(seconds) do
    total_s = trunc(seconds)
    m = div(total_s, 60)
    s = rem(total_s, 60)
    :io_lib.format("~2..0B:~2..0B", [m, s]) |> IO.iodata_to_binary()
  end
end
