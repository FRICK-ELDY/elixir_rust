# Path: lib/engine/scene_behaviour.ex
# Summary: シーンコールバック（init/update/render_type）の behaviour 定義
defmodule Engine.SceneBehaviour do
  @moduledoc """
  シーンコールバックの動作定義。

  各シーンは init/1, update/2, render_type/0 を実装する。
  SceneManager がスタックで管理し、GameEvents が update を呼び出す。
  """

  @doc """
  シーン初期化。init_arg は push_scene / replace_scene で渡された値。
  """
  @callback init(init_arg :: term()) :: {:ok, state :: term()}

  @doc """
  毎 tick 呼ばれる更新。context は GameEvents が構築する共有状態。

  戻り値（最後の opts は省略可。省略時は %{}）:
  - `{:continue, new_state}` または `{:continue, new_state, opts}`
  - `{:transition, :pop, new_state}` または `{:transition, :pop, new_state, opts}`
  - `{:transition, {:push, module(), init_arg}, new_state}` など

  opts の :context_updates で呼び出し元の共有状態を更新できる。
  """
  @callback update(context :: map(), state :: term()) ::
              {:continue, state :: term()}
              | {:continue, state :: term(), opts :: map()}
              | {:transition, :pop, state :: term()}
              | {:transition, :pop, state :: term(), opts :: map()}
              | {:transition, {:push, module(), init_arg :: term()}, state :: term()}
              | {:transition, {:push, module(), init_arg :: term()}, state :: term(), opts :: map()}
              | {:transition, {:replace, module(), init_arg :: term()}, state :: term()}
              | {:transition, {:replace, module(), init_arg :: term()}, state :: term(), opts :: map()}

  @doc """
  描画用のシーン種別。ゲームが任意の atom を定義して返せる。

  FrameCache に格納され、レンダラやテレメトリで描画モード切り替えに利用される。
  例: ヴァンサバは `:playing | :level_up | :boss_alert | :game_over`、
  他ゲームは `:title | :menu | :playing` などを返す。
  """
  @callback render_type() :: atom()
end
