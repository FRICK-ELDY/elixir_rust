defmodule Game.Scenes.LevelUp do
  @moduledoc """
  レベルアップ武器選択シーン。物理演算は一時停止。
  """
  @behaviour Game.SceneBehaviour

  @level_up_auto_select_ms 3_000

  @impl Game.SceneBehaviour
  def init(%{choices: choices, entered_ms: entered_ms} = init_arg) do
    # level はログ表示用。Playing から push 時は常に渡される
    level = Map.get(init_arg, :level)
    {:ok, %{choices: choices, entered_ms: entered_ms, level: level}}
  end

  @impl Game.SceneBehaviour
  def render_type, do: :level_up

  @impl Game.SceneBehaviour
  def update(_context, %{entered_ms: entered_ms} = state) do
    now = System.monotonic_time(:millisecond)
    elapsed = now - entered_ms

    if elapsed >= @level_up_auto_select_ms do
      # タイムアウト → 自動選択でポップ（GameLoop が first choice で処理）
      {:transition, :pop, Map.put(state, :auto_select, true)}
    else
      {:continue, state}
    end
  end
end
