# This file is responsible for configuring your application
# and its dependencies with the aid of the Config module.
#
# Run `mix help config` for more information.

import Config

# Step 34: ゲーム登録メカニズム
# 起動時に「どのゲームを動かすか」を config で指定可能。
# 将来的に `config :game, current: Game.RhythmGame` のように差し替え可能。
config :game, current: Game.VampireSurvivor
