defmodule Game.Scenes.GameOver do
  @moduledoc """
  ゲームオーバーシーン。スコア表示・リトライ待機。
  """
  @behaviour Game.SceneBehaviour

  @impl Game.SceneBehaviour
  def init(_init_arg), do: {:ok, %{}}

  @impl Game.SceneBehaviour
  def render_type, do: :game_over

  @impl Game.SceneBehaviour
  def update(_context, state) do
    # ゲームオーバーは何もしない（リトライは将来的に InputHandler 経由で replace_scene を呼ぶ）
    {:continue, state}
  end
end
