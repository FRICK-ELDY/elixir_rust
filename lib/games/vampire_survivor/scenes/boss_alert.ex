defmodule Game.VampireSurvivor.Scenes.BossAlert do
  @moduledoc """
  ボス出現警告シーン。一定時間後にボスをスポーンして Playing に戻る。
  """
  @behaviour Engine.SceneBehaviour

  require Logger

  @impl Engine.SceneBehaviour
  def init(%{boss_kind: boss_kind, boss_name: boss_name, alert_ms: alert_ms}) do
    {:ok, %{boss_kind: boss_kind, boss_name: boss_name, alert_ms: alert_ms}}
  end

  @impl Engine.SceneBehaviour
  def render_type, do: :boss_alert

  @impl Engine.SceneBehaviour
  def update(context, %{boss_kind: boss_kind, boss_name: boss_name, alert_ms: alert_ms} = state) do
    world_ref = context.world_ref
    now = context.now
    elapsed = now - alert_ms

    if elapsed >= Game.VampireSurvivor.BossSystem.alert_duration_ms() do
      Engine.spawn_boss(world_ref, boss_kind)
      Logger.info("[BOSS] Spawned: #{boss_name}")
      {:transition, :pop, state}
    else
      {:continue, state}
    end
  end
end
