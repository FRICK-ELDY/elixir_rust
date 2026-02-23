defmodule Game.VampireSurvivor do
  @moduledoc """
  Step 34: ゲーム登録メカニズムの Vampire Survivor 実装。

  起動時の初期シーン構成を定義する。将来的には `Game.Engine.Game` behaviour を
  実装し、render_type/0, entity_registry/0 等も提供する（Step 36）。
  """
  @doc """
  起動時のシーンスタック。先頭がルートシーンとなる。

  ## 戻り値

  [scene_spec()] 形式。各 scene_spec は `%{module: module(), init_arg: term()}`。
  """
  @spec initial_scenes() :: [%{module: module(), init_arg: term()}]
  def initial_scenes do
    [
      %{module: Game.Scenes.Playing, init_arg: %{}}
    ]
  end
end
