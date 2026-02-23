# This file is responsible for configuring your application
# and its dependencies with the aid of the Config module.
#
# Run `mix help config` for more information.

import Config

# Step 34: ゲーム登録メカニズム
# 起動時に「どのゲームを動かすか」を config で指定可能。
# Step 40: MiniShooter に切り替え（汎用化検証用）
config :game, current: Game.MiniShooter
# config :game, current: Game.VampireSurvivor

# Step 39: ゲーム別アセットパス
# ゲームの assets_path/0 で上書き可能。未指定時は current ゲームの assets_path を使用。
# GAME_ASSETS_ID 環境変数として game_window 等に渡され、assets/{id}/ を参照する。
# config :game, assets_path: "vampire_survivor"
