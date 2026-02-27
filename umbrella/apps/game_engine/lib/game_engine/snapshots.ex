# Path: umbrella/apps/game_engine/lib/game_engine/snapshots.ex
# Summary: Engine の snapshot_heavy 境界（セーブ/ロード/デバッグ用）
defmodule GameEngine.Snapshots do
  @moduledoc """
  Engine の `snapshot_heavy` 境界。

  セーブ/ロード/デバッグ用途の重いスナップショット取得・適用を集約する。
  毎フレーム呼び出しは禁止し、明示操作時のみ利用する。
  """

  alias GameEngine.Commands
  alias GameEngine.Queries

  def get_save_snapshot(world_ref), do: Queries.get_save_snapshot_heavy(world_ref)
  def load_save_snapshot(world_ref, snapshot), do: Commands.load_save_snapshot(world_ref, snapshot)
end
