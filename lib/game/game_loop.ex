defmodule Game.GameLoop do
  use GenServer
  require Logger

  @tick_ms 16

  def start_link(opts), do: GenServer.start_link(__MODULE__, opts, name: __MODULE__)

  @impl true
  def init(_opts) do
    world_ref = Game.NifBridge.create_world()
    Process.send_after(self(), :tick, @tick_ms)
    {:ok, %{world_ref: world_ref, last_tick: now_ms(), frame_count: 0}}
  end

  @impl true
  def handle_info(:tick, state) do
    delta = now_ms() - state.last_tick
    _frame_id = Game.NifBridge.physics_step(state.world_ref, delta * 1.0)

    if rem(state.frame_count, 60) == 0 do
      Logger.info("Frame: #{state.frame_count}")
    end

    Process.send_after(self(), :tick, @tick_ms)
    {:noreply, %{state | last_tick: now_ms(), frame_count: state.frame_count + 1}}
  end

  defp now_ms, do: System.monotonic_time(:millisecond)
end
