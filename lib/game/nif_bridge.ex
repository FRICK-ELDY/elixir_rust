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
end
