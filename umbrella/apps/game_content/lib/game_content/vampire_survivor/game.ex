# Path: umbrella/apps/game_content/lib/game_content/vampire_survivor/game.ex
# Summary: ヴァンサバの GameEngine.GameBehaviour 実装
defmodule GameContent.VampireSurvivor do
  @behaviour GameEngine.GameBehaviour

  @impl GameEngine.GameBehaviour
  def render_type, do: :playing

  @impl GameEngine.GameBehaviour
  def initial_scenes do
    [
      %{module: GameContent.VampireSurvivor.Scenes.Playing, init_arg: %{}}
    ]
  end

  @impl GameEngine.GameBehaviour
  def entity_registry do
    %{
      enemies: %{slime: 0, bat: 1, golem: 2, skeleton: 3, ghost: 4},
      weapons: %{
        magic_wand: 0, axe: 1, cross: 2, whip: 3, fireball: 4, lightning: 5, garlic: 6
      },
      bosses: %{slime_king: 0, bat_lord: 1, stone_golem: 2}
    }
  end

  @impl GameEngine.GameBehaviour
  def physics_scenes do
    [GameContent.VampireSurvivor.Scenes.Playing]
  end

  @impl GameEngine.GameBehaviour
  def title, do: "Vampire Survivor"

  @impl GameEngine.GameBehaviour
  def version, do: "0.1.0"

  @impl GameEngine.GameBehaviour
  def context_defaults, do: %{}

  @impl GameEngine.GameBehaviour
  def assets_path, do: "vampire_survivor"

  def level_up_scene, do: GameContent.VampireSurvivor.Scenes.LevelUp
  def boss_alert_scene, do: GameContent.VampireSurvivor.Scenes.BossAlert
  def game_over_scene, do: GameContent.VampireSurvivor.Scenes.GameOver
  def wave_label(elapsed_sec), do: GameContent.VampireSurvivor.SpawnSystem.wave_label(elapsed_sec)
  def weapon_label(weapon, level), do: GameContent.VampireSurvivor.LevelSystem.weapon_label(weapon, level)
end
