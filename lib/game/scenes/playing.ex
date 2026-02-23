defmodule Game.Scenes.Playing do
  @moduledoc """
  プレイ中シーン。物理演算・スポーン・ボス/レベルアップチェックを行う。
  """
  @behaviour Game.SceneBehaviour

  require Logger

  @impl Game.SceneBehaviour
  def init(_init_arg), do: {:ok, %{}}

  @impl Game.SceneBehaviour
  def render_type, do: :playing

  @impl Game.SceneBehaviour
  def update(context, state) do
    %{
      world_ref: world_ref,
      now: now,
      elapsed: elapsed,
      last_spawn_ms: last_spawn_ms,
      weapon_levels: weapon_levels,
      spawned_bosses: spawned_bosses,
    } = context

    # 1. ゲームオーバー検知
    if Game.NifBridge.is_player_dead(world_ref) do
      Logger.info("[GAME OVER] Player HP reached 0 at #{div(elapsed, 1000)}s")
      return_transition(:replace, Game.Scenes.GameOver, %{}, state)
    else
      # 2. ボス出現チェック
      elapsed_sec = elapsed / 1000.0
      case Game.BossSystem.check_spawn(elapsed_sec, spawned_bosses) do
        {:spawn, boss_kind, boss_name} ->
          :telemetry.execute([:game, :boss_spawn], %{count: 1}, %{boss: boss_name})
          Logger.info("[BOSS] Alert: #{boss_name} incoming!")
          return_transition(:push, Game.Scenes.BossAlert, %{
            boss_kind: boss_kind,
            boss_name: boss_name,
            alert_ms: now,
          }, state)

        :no_boss ->
          # 3. レベルアップチェック
          {exp, level, level_up_pending, exp_to_next} =
            Game.NifBridge.get_level_up_data(world_ref)

          if level_up_pending do
            :telemetry.execute([:game, :level_up], %{level: level, count: 1}, %{})
            choices = Game.LevelSystem.generate_weapon_choices(weapon_levels)

            if choices == [] do
              Logger.info("[LEVEL UP] All weapons at max level — skipping weapon selection")
              Game.NifBridge.skip_level_up(world_ref)
              {:continue, state}
            else
              choice_labels =
                Enum.map_join(choices, " / ", fn w ->
                  lv = Map.get(weapon_levels, w, 0)
                  Game.LevelSystem.weapon_label(w, lv)
                end)

              Logger.info(
                "[LEVEL UP] Level #{level} -> #{level + 1} | " <>
                  "EXP: #{exp} | to next: #{exp_to_next} | choices: #{choice_labels}"
              )
              Logger.info("[LEVEL UP] Waiting for player selection...")

              return_transition(:push, Game.Scenes.LevelUp, %{
                choices: choices,
                entered_ms: now,
                level: level,
              }, state)
            end
          else
            # 4. 通常スポーン
            new_last_spawn = Game.SpawnSystem.maybe_spawn(world_ref, elapsed, last_spawn_ms)
            {:continue, state, %{context_updates: %{last_spawn_ms: new_last_spawn}}}
          end
      end
    end
  end

  defp return_transition(:push, module, init_arg, state) do
    {:transition, {:push, module, init_arg}, state}
  end

  defp return_transition(:replace, module, init_arg, state) do
    {:transition, {:replace, module, init_arg}, state}
  end
end
