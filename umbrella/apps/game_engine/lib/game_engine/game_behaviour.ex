# Path: umbrella/apps/game_engine/lib/game_engine/game_behaviour.ex
# Summary: ゲームが engine に提供すべき behaviour インターフェース
defmodule GameEngine.GameBehaviour do
  @moduledoc """
  ゲームがエンジンに提供すべきインターフェース。

  エンジンは config で指定されたこの behaviour を実装したモジュールを
  起動時に取得し、初期シーン構築・物理演算対象の判定等に利用する。
  """

  @type scene_spec :: %{module: module(), init_arg: term()}

  @doc "デフォルトのシーン種別。スタックが空のときの fallback 等に使用。"
  @callback render_type() :: atom()

  @doc "起動時のシーンスタック。リストの先頭がスタックの底（ルート）。"
  @callback initial_scenes() :: [scene_spec()]

  @doc "敵・武器の ID → パラメータ。将来のデータ駆動用。"
  @callback entity_registry() :: map()

  @doc "物理演算を実行するシーンモジュールの一覧。"
  @callback physics_scenes() :: [module()]

  @doc "ゲームメタデータ。ウィンドウタイトル等に利用。"
  @callback title() :: String.t()

  @doc "ゲームバージョン。"
  @callback version() :: String.t()

  @doc "コンテキストのデフォルト値。build_context にマージされる。"
  @callback context_defaults() :: map()

  @doc "ゲーム別アセットのサブディレクトリ。"
  @callback assets_path() :: String.t()
end
