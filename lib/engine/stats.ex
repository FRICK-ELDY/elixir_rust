defmodule Engine.Stats do
  @moduledoc """
  1.2.10: ゲームセッション統計をリアルタイム収集する GenServer。

  Elixir の強みを活かし、ゲームロジック（Rust NIF）とは完全に分離した
  プロセスで統計を管理する。クラッシュしても Supervisor が再起動し、
  ゲームプレイには一切影響しない。

  ## 収集する統計
  - 敵タイプ別撃破数
  - 武器別撃破数
  - セッション開始時刻
  - アイテム収集数
  """

  use GenServer
  require Logger

  # ── Public API ────────────────────────────────────────────────

  def start_link(opts \\ []), do: GenServer.start_link(__MODULE__, opts, name: __MODULE__)

  @doc "敵撃破を記録する"
  def record_kill(enemy_kind, weapon_kind) do
    GenServer.cast(__MODULE__, {:kill, enemy_kind, weapon_kind})
  end

  @doc "アイテム収集を記録する"
  def record_item_pickup(item_kind) do
    GenServer.cast(__MODULE__, {:item_pickup, item_kind})
  end

  @doc "レベルアップを記録する"
  def record_level_up(level) do
    GenServer.cast(__MODULE__, {:level_up, level})
  end

  @doc "新しいセッションを開始する（リセット）"
  def new_session do
    GenServer.cast(__MODULE__, :new_session)
  end

  @doc "現在のセッション統計サマリーを返す"
  def session_summary do
    GenServer.call(__MODULE__, :summary)
  end

  # ── GenServer callbacks ────────────────────────────────────────

  @impl true
  def init(_opts) do
    Engine.EventBus.subscribe()
    {:ok, initial_state()}
  end

  @impl true
  def handle_info({:game_events, events}, state) do
    new_state =
      Enum.reduce(events, state, fn
        {:enemy_killed, enemy_kind, weapon_kind}, acc ->
          acc
          |> Map.update(:kills_by_enemy, %{enemy_kind => 1}, &Map.update(&1, enemy_kind, 1, fn n -> n + 1 end))
          |> Map.update(:kills_by_weapon, %{weapon_kind => 1}, &Map.update(&1, weapon_kind, 1, fn n -> n + 1 end))
          |> Map.update(:total_kills, 1, &(&1 + 1))

        {:level_up_event, new_level, _}, acc ->
          Map.put(acc, :max_level_reached, new_level)

        {:item_pickup, item_kind, _}, acc ->
          Map.update(acc, :items_collected, %{item_kind => 1}, &Map.update(&1, item_kind, 1, fn n -> n + 1 end))

        _, acc ->
          acc
      end)

    {:noreply, new_state}
  end

  @impl true
  def handle_cast({:kill, enemy_kind, weapon_kind}, state) do
    kills_by_enemy  = Map.update(state.kills_by_enemy,  enemy_kind,  1, &(&1 + 1))
    kills_by_weapon = Map.update(state.kills_by_weapon, weapon_kind, 1, &(&1 + 1))
    total_kills     = state.total_kills + 1
    {:noreply, %{state |
      kills_by_enemy:  kills_by_enemy,
      kills_by_weapon: kills_by_weapon,
      total_kills:     total_kills,
    }}
  end

  @impl true
  def handle_cast({:item_pickup, item_kind}, state) do
    items = Map.update(state.items_collected, item_kind, 1, &(&1 + 1))
    {:noreply, %{state | items_collected: items}}
  end

  @impl true
  def handle_cast({:level_up, level}, state) do
    {:noreply, %{state | max_level_reached: max(state.max_level_reached, level)}}
  end

  @impl true
  def handle_cast(:new_session, _state) do
    Logger.info("[Stats] 新しいセッションを開始しました")
    {:noreply, initial_state()}
  end

  @impl true
  def handle_call(:summary, _from, state) do
    elapsed_s = (System.monotonic_time(:millisecond) - state.session_start_ms) / 1000.0
    summary = %{
      elapsed_seconds:  elapsed_s,
      total_kills:      state.total_kills,
      kills_by_enemy:   state.kills_by_enemy,
      kills_by_weapon:  state.kills_by_weapon,
      items_collected:  state.items_collected,
      max_level_reached: state.max_level_reached,
    }
    Logger.info("[Stats] セッションサマリー: #{inspect(summary)}")
    {:reply, summary, state}
  end

  # ── Private ───────────────────────────────────────────────────

  defp initial_state do
    %{
      session_start_ms: System.monotonic_time(:millisecond),
      total_kills:      0,
      kills_by_enemy:   %{},
      kills_by_weapon:  %{},
      items_collected:  %{},
      max_level_reached: 1,
    }
  end
end
