# Path: umbrella/apps/game_engine/lib/game_engine/commands.ex
# Summary: Engine の control 境界（World を変更する操作を NifBridge に集約）
defmodule GameEngine.Commands do
  @moduledoc """
  Engine の `control` 境界。

  World を変更する操作（spawn / input / physics / save load 適用など）は
  このモジュール経由で `GameEngine.NifBridge` に集約する。
  """

  alias GameEngine.NifBridge

  def create_world, do: NifBridge.create_world()
  def set_map_obstacles(world_ref, obstacles), do: NifBridge.set_map_obstacles(world_ref, obstacles)

  def create_game_loop_control, do: NifBridge.create_game_loop_control()
  def start_rust_game_loop(world_ref, control_ref, pid), do: NifBridge.start_rust_game_loop(world_ref, control_ref, pid)
  def start_render_thread(world_ref), do: NifBridge.start_render_thread(world_ref)
  def pause_physics(control_ref), do: NifBridge.pause_physics(control_ref)
  def resume_physics(control_ref), do: NifBridge.resume_physics(control_ref)

  def physics_step(world_ref, delta_ms), do: NifBridge.physics_step(world_ref, delta_ms)
  def set_player_input(world_ref, dx, dy), do: NifBridge.set_player_input(world_ref, dx, dy)
  def drain_frame_events(world_ref), do: NifBridge.drain_frame_events(world_ref)

  # 1.10.5: Push 型同期
  def push_tick(world_ref, dx, dy, delta_ms), do: NifBridge.push_tick(world_ref, dx, dy, delta_ms)

  def spawn_enemies(world_ref, kind_id, count), do: NifBridge.spawn_enemies(world_ref, kind_id, count)
  def spawn_elite_enemy(world_ref, kind_id, count, hp_multiplier),
    do: NifBridge.spawn_elite_enemy(world_ref, kind_id, count, hp_multiplier)
  def spawn_boss(world_ref, kind_id), do: NifBridge.spawn_boss(world_ref, kind_id)

  def add_weapon(world_ref, weapon_id), do: NifBridge.add_weapon(world_ref, weapon_id)
  def skip_level_up(world_ref), do: NifBridge.skip_level_up(world_ref)

  def load_save_snapshot(world_ref, snapshot), do: NifBridge.load_save_snapshot(world_ref, snapshot)
end
