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
      {px, py}                        = Game.NifBridge.get_player_pos(state.world_ref)
      {hp, max_hp, score, elapsed_s}  = Game.NifBridge.get_hud_data(state.world_ref)
      enemy_count                     = Game.NifBridge.get_enemy_count(state.world_ref)
      bullet_count                    = Game.NifBridge.get_bullet_count(state.world_ref)
      frame_time_ms                   = Game.NifBridge.get_frame_time_ms(state.world_ref)
      budget_warn                     = if frame_time_ms > @tick_ms, do: " ⚠ OVER BUDGET", else: ""

      hp_bar   = hud_hp_bar(hp, max_hp)
      time_str = format_time(elapsed_s)

      Logger.info(
        "[HUD] #{hp_bar} HP: #{Float.round(hp, 1)}/#{trunc(max_hp)} | " <>
        "Score: #{score} | Time: #{time_str} | " <>
        "Enemies: #{enemy_count} | Bullets: #{bullet_count} | " <>
        "Player: (#{Float.round(px, 1)}, #{Float.round(py, 1)}) | " <>
        "PhysicsTime: #{Float.round(frame_time_ms, 2)}ms#{budget_warn}"
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

  # ── Step 13: HUD ヘルパー ──────────────────────────────────────

  @bar_length 20

  defp hud_hp_bar(hp, max_hp) when max_hp > 0 do
    filled = round(hp / max_hp * @bar_length) |> max(0) |> min(@bar_length)
    empty  = @bar_length - filled
    "[" <> String.duplicate("#", filled) <> String.duplicate("-", empty) <> "]"
  end
  defp hud_hp_bar(_, _), do: "[" <> String.duplicate("-", @bar_length) <> "]"

  defp format_time(seconds) do
    total_s = trunc(seconds)
    m = div(total_s, 60)
    s = rem(total_s, 60)
    :io_lib.format("~2..0B:~2..0B", [m, s]) |> IO.iodata_to_binary()
  end
end
