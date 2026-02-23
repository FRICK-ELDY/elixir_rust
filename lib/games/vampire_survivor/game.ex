defmodule Game.VampireSurvivor do
  @moduledoc """
  Step 35–36: ヴァンサバを Game 実装として分離し、Engine.Game behaviour を実装。

  起動時の初期シーン構成、物理演算対象シーン、シーン遷移で使用する
  モジュール参照を提供する。
  """
  @behaviour Engine.Game

  # ── Engine.Game callbacks ───────────────────────────────────────

  @impl Engine.Game
  def render_type, do: :playing

  @impl Engine.Game
  def initial_scenes do
    [
      %{module: Game.VampireSurvivor.Scenes.Playing, init_arg: %{}}
    ]
  end

  @impl Engine.Game
  def entity_registry, do: %{}

  @impl Engine.Game
  def physics_scenes do
    [Game.VampireSurvivor.Scenes.Playing]
  end

  @impl Engine.Game
  def title, do: "Vampire Survivor"

  @impl Engine.Game
  def version, do: "0.1.0"

  @impl Engine.Game
  def context_defaults, do: %{}

  # ── Vampire Survivor 固有（シーン遷移等で GameLoop が参照）──

  @doc "レベルアップ武器選択シーンのモジュール"
  def level_up_scene, do: Game.VampireSurvivor.Scenes.LevelUp

  @doc "ゲームオーバーシーンのモジュール"
  def game_over_scene, do: Game.VampireSurvivor.Scenes.GameOver

  @doc "StressMonitor / GameLoop のログ用ウェーブラベル"
  def wave_label(elapsed_sec), do: Game.VampireSurvivor.SpawnSystem.wave_label(elapsed_sec)

  @doc "武器の表示用ラベル（ログ用）"
  def weapon_label(weapon, level), do: Game.VampireSurvivor.LevelSystem.weapon_label(weapon, level)
end
