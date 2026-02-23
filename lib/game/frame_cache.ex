defmodule Game.FrameCache do
  @moduledoc """
  フレームごとのゲーム状態スナップショットを ETS に書き込む。

  ETS の特性:
  - 書き込みは GameLoop（単一ライター）のみ — 競合なし
  - 読み取りは任意のプロセスからロックフリーで可能
  - read_concurrency: true で並列読み取りを最適化
  - GameLoop がクラッシュして ETS テーブルが消えても、
    Supervisor 再起動後に GameLoop.init/1 で再作成される
  """

  @table :frame_cache

  @doc "GameLoop.init/1 から呼ぶ — ETS テーブルを作成する"
  def init do
    :ets.new(@table, [:named_table, :public, :set, read_concurrency: true])
  end

  @doc "GameLoop が毎秒（60 フレームごと）書き込む"
  def put(enemy_count, bullet_count, physics_ms, hud_data, render_type \\ :playing) do
    :ets.insert(@table, {:snapshot, %{
      enemy_count:  enemy_count,
      bullet_count: bullet_count,
      physics_ms:   physics_ms,
      hud_data:     hud_data,
      render_type:  render_type,
      updated_at:   System.monotonic_time(:millisecond),
    }})
  end

  @doc "StressMonitor など任意のプロセスがロックフリーで読み取る"
  def get do
    case :ets.lookup(@table, :snapshot) do
      [{:snapshot, data}] -> {:ok, data}
      []                  -> :empty
    end
  end
end
