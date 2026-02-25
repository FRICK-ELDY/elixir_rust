# Path: lib/engine/event_bus.ex
# Summary: フレームイベントをサブスクライバーに配信する GenServer
defmodule Engine.EventBus do
  @moduledoc """
  1.3.1: フレームイベントを受け取り、複数のサブスクライバーに配信する。

  Elixir/OTP の強みを体現するモジュール:
  - プロセスへの send はノンブロッキング — GameEvents を止めない
  - サブスクライバーが重い処理をしても GameEvents に影響しない
  - Process.monitor でサブスクライバーの死活を自動監視
  - EventBus 自体がクラッシュしても Supervisor が即座に再起動し、
    GameEvents は一切影響を受けない（one_for_one 戦略）
  """

  use GenServer
  require Logger

  # ── Public API ────────────────────────────────────────────────

  def start_link(opts \\ []), do: GenServer.start_link(__MODULE__, opts, name: __MODULE__)

  @doc "呼び出したプロセスをサブスクライバーとして登録する"
  def subscribe(pid \\ self()) do
    GenServer.cast(__MODULE__, {:subscribe, pid})
  end

  @doc "イベントリストを全サブスクライバーにブロードキャストする"
  def broadcast(events) when is_list(events) do
    GenServer.cast(__MODULE__, {:broadcast, events})
  end

  # ── GenServer callbacks ────────────────────────────────────────

  @impl true
  def init(_opts) do
    {:ok, %{subscribers: MapSet.new()}}
  end

  @impl true
  def handle_cast({:subscribe, pid}, state) do
    Process.monitor(pid)
    Logger.debug("[EventBus] Subscriber registered: #{inspect(pid)}")
    {:noreply, %{state | subscribers: MapSet.put(state.subscribers, pid)}}
  end

  @impl true
  def handle_cast({:broadcast, events}, state) do
    Enum.each(state.subscribers, fn pid ->
      send(pid, {:game_events, events})
    end)
    {:noreply, state}
  end

  @impl true
  def handle_info({:DOWN, _ref, :process, pid, _reason}, state) do
    Logger.debug("[EventBus] Subscriber down: #{inspect(pid)}")
    {:noreply, %{state | subscribers: MapSet.delete(state.subscribers, pid)}}
  end
end
