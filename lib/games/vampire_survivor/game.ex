defmodule Game.VampireSurvivor do
  @moduledoc """
  Step 35: ヴァンサバを Game 実装として分離。

  起動時の初期シーン構成、物理演算対象シーン、シーン遷移で使用する
  モジュール参照を提供する。Step 36 で Engine.Game behaviour を実装予定。
  """

  @doc """
  起動時のシーンスタック。リストの先頭の要素がスタックの底（ルート）になります。
  """
  @spec initial_scenes() :: [%{module: module(), init_arg: term()}]
  def initial_scenes do
    [
      %{module: Game.VampireSurvivor.Scenes.Playing, init_arg: %{}}
    ]
  end

  @doc """
  物理演算を実行するシーンモジュールの一覧。
  """
  @spec physics_scenes() :: [module()]
  def physics_scenes do
    [Game.VampireSurvivor.Scenes.Playing]
  end

  @doc "レベルアップ武器選択シーンのモジュール"
  def level_up_scene, do: Game.VampireSurvivor.Scenes.LevelUp

  @doc "ゲームオーバーシーンのモジュール"
  def game_over_scene, do: Game.VampireSurvivor.Scenes.GameOver

  @doc "StressMonitor / GameLoop のログ用ウェーブラベル"
  def wave_label(elapsed_sec), do: Game.VampireSurvivor.SpawnSystem.wave_label(elapsed_sec)

  @doc "武器の表示用ラベル（ログ用）"
  def weapon_label(weapon, level), do: Game.VampireSurvivor.LevelSystem.weapon_label(weapon, level)
end
