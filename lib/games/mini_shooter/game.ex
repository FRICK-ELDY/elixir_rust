defmodule Game.MiniShooter do
  @moduledoc """
  Step 40: 2 つ目のゲーム（ミニマル実装）。

  汎用化検証用の極小ゲーム。「タイトル → プレイ（敵が直進するだけ）→ ゲームオーバー」の流れのみ。
  """
  @behaviour Engine.Game

  # ── Engine.Game callbacks ───────────────────────────────────────

  @impl Engine.Game
  def render_type, do: :playing

  @impl Engine.Game
  def initial_scenes do
    [
      %{module: Game.MiniShooter.Scenes.Playing, init_arg: %{}}
    ]
  end

  @impl Engine.Game
  def entity_registry do
    %{
      enemies: %{slime: 0},
      weapons: %{magic_wand: 0},
      bosses: %{slime_king: 0}
    }
  end

  @impl Engine.Game
  def physics_scenes do
    [Game.MiniShooter.Scenes.Playing]
  end

  @impl Engine.Game
  def title, do: "Mini Shooter"

  @impl Engine.Game
  def version, do: "0.1.0"

  @impl Engine.Game
  def context_defaults, do: %{}

  @impl Engine.Game
  def assets_path, do: "mini_shooter"

  # ── GameLoop / StressMonitor が参照するオプションAPI ──────────────
  # ヴァンサバと同様のインターフェース。レベルアップなしのためスタブを返す。

  def level_up_scene, do: __MODULE__
  def boss_alert_scene, do: __MODULE__
  def game_over_scene, do: Game.MiniShooter.Scenes.GameOver
  def wave_label(_elapsed_sec), do: "Mini"
  def weapon_label(_weapon, _level), do: "n/a"
end
