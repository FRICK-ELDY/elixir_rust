defmodule Engine.Game do
  @moduledoc """
  ゲームがエンジンに提供すべきインターフェース（Step 32 設計）。

  エンジンは config で指定されたこの behaviour を実装したモジュールを
  起動時に取得し、初期シーン構築・物理演算対象の判定等に利用する。
  """

  @type scene_spec :: %{module: module(), init_arg: term()}

  @doc """
  デフォルトのシーン種別。スタックが空のときの fallback 等に使用。
  各シーンは SceneBehaviour.render_type/0 で独自の atom を返す。
  """
  @callback render_type() :: atom()

  @doc """
  起動時のシーンスタック。リストの先頭がスタックの底（ルート）。
  """
  @callback initial_scenes() :: [scene_spec()]

  @doc """
  敵・武器の ID → パラメータ。将来のデータ駆動用。現状は空 map を返す。
  """
  @callback entity_registry() :: map()

  @doc """
  物理演算を実行するシーンモジュールの一覧。
  GameEvents の maybe_run_physics がこの一覧に含まれるシーンのときのみ physics_step を実行する。
  """
  @callback physics_scenes() :: [module()]

  @doc "ゲームメタデータ。ウィンドウタイトル等に利用。"
  @callback title() :: String.t()

  @doc "ゲームバージョン。"
  @callback version() :: String.t()

  @doc """
  コンテキストのデフォルト値。build_context にマージされる。
  wave, difficulty 等ゲーム固有の値を追加可能。
  """
  @callback context_defaults() :: map()

  @doc """
  ゲーム別アセットのサブディレクトリ（Step 39）。
  `assets/` 直下のサブディレクトリ名を返す。
  例: `"vampire_survivor"` → `assets/vampire_survivor/sprites/atlas.png` を参照
  空文字列の場合は `assets/sprites/atlas.png` を参照（従来どおり）。
  """
  @callback assets_path() :: String.t()
end
