defmodule Game.GameLoop do
  use GenServer
  require Logger

  @tick_ms 16

  def start_link(opts), do: GenServer.start_link(__MODULE__, opts, name: __MODULE__)

  @impl true
  def init(_opts) do
    world_ref = Game.NifBridge.create_world()
    start_ms = now_ms()
    Process.send_after(self(), :tick, @tick_ms)

    {:ok,
     %{
       world_ref:     world_ref,
       last_tick:     start_ms,
       frame_count:   0,
       start_ms:      start_ms,
       last_spawn_ms: start_ms
     }}
  end

  @impl true
  def handle_cast({:input, :move, {dx, dy}}, state) do
    Game.NifBridge.set_player_input(state.world_ref, dx * 1.0, dy * 1.0)
    {:noreply, state}
  end

  @impl true
  def handle_info(:tick, state) do
    now     = now_ms()
    delta   = now - state.last_tick
    elapsed = now - state.start_ms

    _frame_id = Game.NifBridge.physics_step(state.world_ref, delta * 1.0)

    # 敵スポーン（2 秒ごとに 10 体）
    new_last_spawn =
      Game.SpawnSystem.maybe_spawn(state.world_ref, elapsed, state.last_spawn_ms)

    if rem(state.frame_count, 60) == 0 do
      {px, py}    = Game.NifBridge.get_player_pos(state.world_ref)
      hp          = Game.NifBridge.get_player_hp(state.world_ref)
      render_data = Game.NifBridge.get_render_data(state.world_ref)
      enemy_count = length(render_data) - 1
      Logger.info(
        "Frame: #{state.frame_count} | " <>
        "Player: (#{Float.round(px, 1)}, #{Float.round(py, 1)}) | " <>
        "HP: #{Float.round(hp, 1)} | " <>
        "Enemies: #{enemy_count}"
      )
    end

    Process.send_after(self(), :tick, @tick_ms)

    {:noreply,
     %{state |
       last_tick:     now,
       frame_count:   state.frame_count + 1,
       last_spawn_ms: new_last_spawn
     }}
  end

  defp now_ms, do: System.monotonic_time(:millisecond)
end
