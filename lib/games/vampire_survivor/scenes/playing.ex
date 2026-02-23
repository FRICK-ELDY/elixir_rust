defmodule Game.VampireSurvivor.Scenes.Playing do
  @moduledoc """
  プレイ中シーン。物理演算・スポーン・ボス/レベルアップチェックを行う。
  """
  @behaviour Engine.SceneBehaviour

  require Logger

  @impl Engine.SceneBehaviour
  def init(_init_arg), do: {:ok, %{spawned_bosses: []}}

  @impl Engine.SceneBehaviour
  def render_type, do: :playing

  @impl Engine.SceneBehaviour
  def update(context, state) do
    %{
      world_ref: world_ref,
      now: now,
      elapsed: elapsed,
      last_spawn_ms: last_spawn_ms,
      weapon_levels: weapon_levels,
    } = context

    spawned_bosses = Map.get(state, :spawned_bosses, [])

    if Game.NifBridge.is_player_dead(world_ref) do
      Logger.info("[GAME OVER] Player HP reached 0 at #{div(elapsed, 1000)}s")
      return_transition(:replace, Game.VampireSurvivor.Scenes.GameOver, %{}, state)
    else
      elapsed_sec = elapsed / 1000.0

      case Game.VampireSurvivor.BossSystem.check_spawn(elapsed_sec, spawned_bosses) do
        {:spawn, boss_kind, boss_name} ->
          :telemetry.execute([:game, :boss_spawn], %{count: 1}, %{boss: boss_name})
          Logger.info("[BOSS] Alert: #{boss_name} incoming!")
          new_state = %{state | spawned_bosses: [boss_kind | spawned_bosses]}
          return_transition(:push, Game.VampireSurvivor.Scenes.BossAlert, %{
            boss_kind: boss_kind,
            boss_name: boss_name,
            alert_ms: now,
          }, new_state)

        :no_boss ->
          {exp, level, level_up_pending, exp_to_next} =
            Game.NifBridge.get_level_up_data(world_ref)

          if level_up_pending do
            :telemetry.execute([:game, :level_up], %{level: level, count: 1}, %{})
            choices = Game.VampireSurvivor.LevelSystem.generate_weapon_choices(weapon_levels)

            if choices == [] do
              Logger.info("[LEVEL UP] All weapons at max level — skipping weapon selection")
              Game.NifBridge.skip_level_up(world_ref)
              {:continue, state}
            else
              choice_labels =
                Enum.map_join(choices, " / ", fn w ->
                  lv = Map.get(weapon_levels, w, 0)
                  Game.VampireSurvivor.LevelSystem.weapon_label(w, lv)
                end)

              Logger.info(
                "[LEVEL UP] Level #{level} -> #{level + 1} | " <>
                  "EXP: #{exp} | to next: #{exp_to_next} | choices: #{choice_labels}"
              )
              Logger.info("[LEVEL UP] Waiting for player selection...")

              return_transition(:push, Game.VampireSurvivor.Scenes.LevelUp, %{
                choices: choices,
                entered_ms: now,
                level: level,
              }, state)
            end
          else
            new_last_spawn = Game.VampireSurvivor.SpawnSystem.maybe_spawn(world_ref, elapsed, last_spawn_ms)
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
