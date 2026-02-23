defmodule Game.SceneManager do
  @moduledoc """
  シーンスタックを管理する GenServer。

  シーンは `%{module: module(), state: term()}` で表現し、
  push / pop によりスタックで管理する。ステージ選択・設定画面・
  チュートリアル等の追加が容易になる。

  現在のシーン種別は `render_type/0` で取得でき、
  Rust 側の描画（GamePhase）に渡すことができる。
  """

  use GenServer

  # ── Public API ──────────────────────────────────────────────────

  def start_link(opts), do: GenServer.start_link(__MODULE__, opts, name: __MODULE__)

  @doc "現在のシーンを返す。スタックが空の場合は :empty"
  def current do
    GenServer.call(__MODULE__, :current)
  end

  @doc "描画用の現在シーン種別（任意の atom）。スタックが空のときは初期シーンの値を返す。"
  def render_type do
    GenServer.call(__MODULE__, :render_type)
  end

  @doc "新規シーンをスタックにプッシュ"
  def push_scene(module, init_arg \\ %{}) do
    GenServer.call(__MODULE__, {:push, module, init_arg})
  end

  @doc """
  現在のシーンをポップ。オーバーレイ（LevelUp, BossAlert）の戻り用。
  ルートシーンのみの場合はポップ不可。GameOver からのリスタートは
  `replace_scene/2` を使用すること。
  """
  def pop_scene do
    GenServer.call(__MODULE__, :pop)
  end

  @doc "現在のシーンを別シーンに置換。GameOver への遷移・リスタート時に使用"
  def replace_scene(module, init_arg \\ %{}) do
    GenServer.call(__MODULE__, {:replace, module, init_arg})
  end

  @doc "現在シーンの state を更新する"
  def update_current(fun) when is_function(fun, 1) do
    GenServer.call(__MODULE__, {:update_current, fun})
  end

  # ── GenServer callbacks ─────────────────────────────────────────

  @impl true
  def init(_opts) do
    # 初期シーン: Playing（ゲーム開始）。将来タイトル画面にする場合はここを変更
    {:ok, scene} = init_scene(Game.Scenes.Playing, %{})
    default_render_type = scene[:module].render_type()
    state = %{stack: [scene], default_render_type: default_render_type}
    {:ok, state}
  end

  @impl true
  def handle_call(:current, _from, %{stack: []} = state) do
    {:reply, :empty, state}
  end

  def handle_call(:current, _from, %{stack: [top | _]} = state) do
    {:reply, {:ok, top}, state}
  end

  def handle_call(:render_type, _from, %{stack: [], default_render_type: default} = state) do
    # スタックが空のときは初期シーンの render_type を返す（通常は到達しない）
    {:reply, default, state}
  end

  def handle_call(:render_type, _from, %{stack: [%{module: mod} | _]} = state) do
    {:reply, mod.render_type(), state}
  end

  def handle_call({:push, module, init_arg}, _from, %{stack: stack} = state) do
    {:ok, scene} = init_scene(module, init_arg)
    {:reply, :ok, %{state | stack: [scene | stack]}}
  end

  def handle_call(:pop, _from, %{stack: [_]} = state) do
    # ルートのみの場合はポップ不可。GameOver 等からの戻りは replace_scene を使用
    {:reply, {:error, :cannot_pop_root}, state}
  end

  def handle_call(:pop, _from, %{stack: [_top | rest]} = state) do
    {:reply, :ok, %{state | stack: rest}}
  end

  def handle_call({:replace, module, init_arg}, _from, %{stack: [_ | rest]} = state) do
    {:ok, scene} = init_scene(module, init_arg)
    {:reply, :ok, %{state | stack: [scene | rest]}}
  end

  def handle_call({:replace, module, init_arg}, _from, %{stack: []} = state) do
    {:ok, scene} = init_scene(module, init_arg)
    {:reply, :ok, %{state | stack: [scene]}}
  end

  def handle_call({:update_current, fun}, _from, %{stack: [top | rest]} = state) do
    new_state = fun.(top.state)
    new_top = %{top | state: new_state}
    {:reply, :ok, %{state | stack: [new_top | rest]}}
  end

  defp init_scene(module, init_arg) do
    {:ok, scene_state} = module.init(init_arg)
    {:ok, %{module: module, state: scene_state}}
  end
end
