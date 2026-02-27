# Path: umbrella/apps/game_content/lib/game_content/mini_shooter/game.ex
# Summary: ミニシューターの GameEngine.GameBehaviour 実装
defmodule GameContent.MiniShooter do
  @behaviour GameEngine.GameBehaviour

  @impl GameEngine.GameBehaviour
  def render_type, do: :playing

  @impl GameEngine.GameBehaviour
  def initial_scenes do
    [
      %{module: GameContent.MiniShooter.Scenes.Playing, init_arg: %{}}
    ]
  end

  @impl GameEngine.GameBehaviour
  def entity_registry do
    %{
      enemies: %{slime: 0},
      weapons: %{magic_wand: 0},
      bosses: %{slime_king: 0}
    }
  end

  @impl GameEngine.GameBehaviour
  def physics_scenes do
    [GameContent.MiniShooter.Scenes.Playing]
  end

  @impl GameEngine.GameBehaviour
  def title, do: "Mini Shooter"

  @impl GameEngine.GameBehaviour
  def version, do: "0.1.0"

  @impl GameEngine.GameBehaviour
  def context_defaults, do: %{}

  @impl GameEngine.GameBehaviour
  def assets_path, do: "mini_shooter"

  def level_up_scene, do: __MODULE__
  def boss_alert_scene, do: __MODULE__
  def game_over_scene, do: GameContent.MiniShooter.Scenes.GameOver
  def wave_label(_elapsed_sec), do: "Mini"
  def weapon_label(_weapon, _level), do: "n/a"
end
