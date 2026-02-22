defmodule Game.NifBridge do
  @moduledoc """
  Rust NIF のラッパーモジュール。
  `use Rustler` により、コンパイル時に `native/game_native` クレートが
  自動的にビルドされ、`.dll` がロードされる。
  """

  use Rustler,
    otp_app: :game,
    crate: :game_native

  def add(_a, _b), do: :erlang.nif_error(:nif_not_loaded)
  def create_world(), do: :erlang.nif_error(:nif_not_loaded)
  def physics_step(_world, _delta_ms), do: :erlang.nif_error(:nif_not_loaded)

  # Step 8: プレイヤー入力・座標取得
  def set_player_input(_world, _dx, _dy), do: :erlang.nif_error(:nif_not_loaded)
  def get_player_pos(_world), do: :erlang.nif_error(:nif_not_loaded)

  # Step 9: 敵スポーン + 描画データ取得
  def spawn_enemies(_world, _kind, _count), do: :erlang.nif_error(:nif_not_loaded)
  def get_render_data(_world), do: :erlang.nif_error(:nif_not_loaded)
end
