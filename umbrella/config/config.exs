# Path: umbrella/config/config.exs
# Summary: Umbrella 共通設定（game_engine / game_content の設定を集約）
import Config

# Rustler NIF ビルドパス（絶対パスで指定）
# nif_bridge.ex 側で __DIR__ から動的に解決するため、ここでは設定不要
# config :game_engine, GameEngine.NifBridge, path: "..."

# 起動ゲーム選択（game_content アプリのモジュールを指定）
config :game_engine, current: GameContent.VampireSurvivor
# config :game_engine, current: GameContent.MiniShooter

# マップ障害物設定: :plain | :forest | :minimal
config :game_engine, map: :minimal

# tick_hz: 10 | 20 | 30（デフォルト: 20Hz）
# GAME_ENGINE_HEADLESS=true 環境変数でヘッドレスモードに切り替え可能
headless_env = System.get_env("GAME_ENGINE_HEADLESS", "false") == "true"
config :game_engine, tick_hz: 20, headless: headless_env

# サーバー用（config/prod.exs で上書き、または GAME_ENGINE_HEADLESS=true で起動）
# config :game_engine, tick_hz: 20, headless: true
