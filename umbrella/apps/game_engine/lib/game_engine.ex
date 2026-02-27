# Path: umbrella/apps/game_engine/lib/game_engine.ex
# Summary: ゲームエンジンの安定化された公開 API
defmodule GameEngine do
  @moduledoc """
  ゲームエンジンの安定化された公開 API。

  ゲームは **GameEngine モジュール経由でのみ** エンジンとやり取りする。
  """

  alias GameEngine.Commands
  alias GameEngine.Queries

  # ── World 操作 ─────────────────────────────────────────────────────

  def spawn_enemies(world_ref, kind, count) do
    kind_id = resolve_enemy_id(kind)
    Commands.spawn_enemies(world_ref, kind_id, count)
  end

  def spawn_elite_enemy(world_ref, kind, count, hp_multiplier) do
    kind_id = resolve_enemy_id(kind)
    Commands.spawn_elite_enemy(world_ref, kind_id, count, hp_multiplier)
  end

  def spawn_boss(world_ref, kind) do
    kind_id = resolve_boss_id(kind)
    Commands.spawn_boss(world_ref, kind_id)
  end

  def get_enemy_count(world_ref), do: Queries.get_enemy_count(world_ref)

  def is_player_dead?(world_ref), do: Queries.is_player_dead(world_ref)

  def get_level_up_data(world_ref), do: Queries.get_level_up_data(world_ref)

  def skip_level_up(world_ref), do: Commands.skip_level_up(world_ref)

  # ── エンジン内部用 ──────────────────────────────────────────────────

  def create_world, do: Commands.create_world()

  def set_map_obstacles(world_ref, obstacles), do: Commands.set_map_obstacles(world_ref, obstacles)

  def create_game_loop_control, do: Commands.create_game_loop_control()

  def start_rust_game_loop(world_ref, control_ref, pid) do
    Commands.start_rust_game_loop(world_ref, control_ref, pid)
  end

  def start_render_thread(world_ref), do: Commands.start_render_thread(world_ref)

  def pause_physics(control_ref), do: Commands.pause_physics(control_ref)

  def resume_physics(control_ref), do: Commands.resume_physics(control_ref)

  def physics_step(world_ref, delta_ms), do: Commands.physics_step(world_ref, delta_ms)

  def set_player_input(world_ref, dx, dy), do: Commands.set_player_input(world_ref, dx, dy)

  def drain_frame_events(world_ref), do: Commands.drain_frame_events(world_ref)

  # 1.10.5: Push 型同期（Elixir → Rust 入力 → delta 返却）
  def push_tick(world_ref, dx, dy, delta_ms), do: Commands.push_tick(world_ref, dx, dy, delta_ms)

  def get_frame_metadata(world_ref), do: Queries.get_frame_metadata(world_ref)

  def add_weapon(world_ref, weapon_name) when is_binary(weapon_name) do
    weapon_id = resolve_weapon_id(String.to_atom(weapon_name))
    Commands.add_weapon(world_ref, weapon_id)
  end

  def add_weapon(world_ref, weapon) when is_atom(weapon) do
    weapon_id = resolve_weapon_id(weapon)
    Commands.add_weapon(world_ref, weapon_id)
  end

  def get_weapon_levels(world_ref), do: Queries.get_weapon_levels(world_ref)

  # ── セーブ・ロード ──────────────────────────────────────────────────

  def save_session(world_ref), do: GameEngine.SaveManager.save_session(world_ref)

  def load_session(world_ref), do: GameEngine.SaveManager.load_session(world_ref)

  def has_save?, do: GameEngine.SaveManager.has_save?()

  def save_high_score(score), do: GameEngine.SaveManager.save_high_score(score)

  def load_high_scores, do: GameEngine.SaveManager.load_high_scores()

  def best_score, do: GameEngine.SaveManager.best_score()

  # ── ルーム管理 ──────────────────────────────────────────────────────

  def start_room(room_id), do: GameEngine.RoomSupervisor.start_room(room_id)

  def stop_room(room_id), do: GameEngine.RoomSupervisor.stop_room(room_id)

  def list_rooms, do: GameEngine.RoomSupervisor.list_rooms()

  def get_loop_for_room(room_id), do: GameEngine.RoomRegistry.get_loop(room_id)

  # ── シーン操作 ──────────────────────────────────────────────────────

  defdelegate push_scene(module, init_arg \\ %{}), to: GameEngine.SceneManager
  defdelegate pop_scene(), to: GameEngine.SceneManager
  defdelegate replace_scene(module, init_arg \\ %{}), to: GameEngine.SceneManager
  defdelegate current_scene(), to: GameEngine.SceneManager, as: :current
  defdelegate render_type(), to: GameEngine.SceneManager
  defdelegate update_current_scene(fun), to: GameEngine.SceneManager, as: :update_current

  # ── entity_registry による ID 解決（内部用）────────────────────────

  defp resolve_enemy_id(kind) do
    game = Application.get_env(:game_engine, :current)
    Map.fetch!(game.entity_registry().enemies, kind)
  end

  defp resolve_boss_id(kind) do
    game = Application.get_env(:game_engine, :current)
    Map.fetch!(game.entity_registry().bosses, kind)
  end

  defp resolve_weapon_id(weapon) do
    game = Application.get_env(:game_engine, :current)
    Map.fetch!(game.entity_registry().weapons, weapon)
  end
end
