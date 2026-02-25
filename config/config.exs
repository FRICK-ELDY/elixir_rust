# Path: config/config.exs
# Summary: アプリケーション設定（ゲーム選択・マップ・Telemetry 等）
# This file is responsible for configuring your application
# and its dependencies with the aid of the Config module.
#
# Run `mix help config` for more information.

import Config

# 1.6.8: Rustler NIF ビルドパス（1.7.7 で game_window 廃止後も game_native を明示）
# プロジェクトルートからの相対パス。workspace 化後も Mix が native/game_native を正しくビルドするよう path を指定する。
config :game, App.NifBridge,
  path: "native/game_native"

# 1.4.3: ゲーム登録メカニズム
# 起動時に「どのゲームを動かすか」を config で指定可能。
# 1.4.9: MiniShooter に切り替え（汎用化検証用）
config :game, current: Game.MiniShooter
# config :game, current: Game.VampireSurvivor

# 1.5.2: マップ障害物。:plain | :forest | :minimal
config :game, map: :plain

# 1.4.8: ゲーム別アセットパス
# ゲームの assets_path/0 で上書き可能。未指定時は current ゲームの assets_path を使用。
# GAME_ASSETS_ID 環境変数として game_native 等に渡され、assets/{id}/ を参照する。
# config :game, assets_path: "vampire_survivor"
