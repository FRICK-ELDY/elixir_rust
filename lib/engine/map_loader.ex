defmodule Engine.MapLoader do
  @moduledoc """
  1.5.2: マップ障害物の定義。

  マップ ID に応じて障害物リストを返す。
  各要素は `{x, y, radius, kind}` のタプル。
  - kind: 0 = 木, 1 = 岩（将来: Ghost のすり抜け判定用）
  """

  @doc """
  マップ ID に応じて障害物リストを返す。

  ## 戻り値
  `[{x, y, radius, kind}, ...]` のリスト。
  kind: 0=木, 1=岩

  ## 例
      Engine.MapLoader.obstacles_for_map(:plain)
      # => []

      Engine.MapLoader.obstacles_for_map(:forest)
      # => [{512, 512, 40, 0}, {1024, 768, 30, 1}, ...]
  """
  def obstacles_for_map(:plain), do: []

  def obstacles_for_map(:forest) do
    # 木・岩の配置データ（x, y, radius, kind）
    # マップサイズ 4096x4096 を想定した例
    [
      {512.0, 512.0, 40.0, 0},
      {1024.0, 768.0, 30.0, 1},
      {1536.0, 256.0, 40.0, 0},
      {768.0, 1024.0, 35.0, 1},
      {2048.0, 2048.0, 50.0, 0},
      {2560.0, 1280.0, 30.0, 1},
      {1280.0, 2560.0, 40.0, 0},
      {640.0, 640.0, 25.0, 1}
    ]
  end

  def obstacles_for_map(:minimal) do
    # 動作確認用: 画面中央付近に数個
    [
      {512.0, 360.0, 40.0, 0},
      {768.0, 400.0, 30.0, 1}
    ]
  end

  def obstacles_for_map(_unknown), do: []
end
