defmodule Engine.Queries do
  @moduledoc """
  Engine の query 境界。

  World を変更しない取得系操作をこのモジュールに集約する。
  """

  alias App.NifBridge

  def get_enemy_count(world_ref), do: NifBridge.get_enemy_count(world_ref)
  def is_player_dead(world_ref), do: NifBridge.is_player_dead(world_ref)
  def get_level_up_data(world_ref), do: NifBridge.get_level_up_data(world_ref)
  def get_frame_metadata(world_ref), do: NifBridge.get_frame_metadata(world_ref)
  def get_weapon_levels(world_ref), do: NifBridge.get_weapon_levels(world_ref)
  def get_save_snapshot(world_ref), do: NifBridge.get_save_snapshot(world_ref)
end
