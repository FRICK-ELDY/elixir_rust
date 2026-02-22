defmodule Game.InputHandler do
  @moduledoc """
  キー入力状態を管理し、移動ベクトルを GameLoop に通知する GenServer。
  """
  use GenServer

  def start_link(opts), do: GenServer.start_link(__MODULE__, opts, name: __MODULE__)

  @impl true
  def init(_opts), do: {:ok, %{keys_held: MapSet.new()}}

  def key_down(key), do: GenServer.cast(__MODULE__, {:key_down, key})
  def key_up(key),   do: GenServer.cast(__MODULE__, {:key_up, key})

  @impl true
  def handle_cast({:key_down, key}, state) do
    if MapSet.member?(state.keys_held, key) do
      {:noreply, state}
    else
      new_keys = MapSet.put(state.keys_held, key)
      notify_game_loop(new_keys)
      {:noreply, %{state | keys_held: new_keys}}
    end
  end

  @impl true
  def handle_cast({:key_up, key}, state) do
    if MapSet.member?(state.keys_held, key) do
      new_keys = MapSet.delete(state.keys_held, key)
      notify_game_loop(new_keys)
      {:noreply, %{state | keys_held: new_keys}}
    else
      {:noreply, state}
    end
  end

  defp notify_game_loop(keys_held) do
    dx =
      (if MapSet.member?(keys_held, :d) or MapSet.member?(keys_held, :arrow_right), do: 1, else: 0) +
      (if MapSet.member?(keys_held, :a) or MapSet.member?(keys_held, :arrow_left),  do: -1, else: 0)

    dy =
      (if MapSet.member?(keys_held, :s) or MapSet.member?(keys_held, :arrow_down), do: 1, else: 0) +
      (if MapSet.member?(keys_held, :w) or MapSet.member?(keys_held, :arrow_up),   do: -1, else: 0)

    GenServer.cast(Game.GameLoop, {:input, :move, {dx, dy}})
  end
end
