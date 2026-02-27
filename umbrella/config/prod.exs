# Path: umbrella/config/prod.exs
# Summary: 本番サーバー用設定（headless: true・Phoenix Endpoint）
import Config

# ヘッドレス: Rust スレッド（描画・音）をロードしない
config :game_engine,
  tick_hz: 20,
  headless: true

# Phoenix Endpoint
config :game_network, GameNetwork.Endpoint,
  url: [host: "example.com"],
  http: [port: 4000],
  secret_key_base: System.get_env("SECRET_KEY_BASE") || raise("SECRET_KEY_BASE is not set")
