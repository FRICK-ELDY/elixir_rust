# Path: umbrella/apps/game_engine/mix.exs
# Summary: game_engine アプリ（エンジンコア: NIF・tick_hz・ルーム管理）
defmodule GameEngine.MixProject do
  use Mix.Project

  def project do
    [
      app: :game_engine,
      version: "0.1.0",
      build_path: "../../_build",
      config_path: "../../config/config.exs",
      deps_path: "../../deps",
      lockfile: "../../mix.lock",
      elixir: "~> 1.19",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  def application do
    [
      extra_applications: [:logger],
      mod: {GameEngine.Application, []}
    ]
  end

  defp deps do
    [
      {:rustler, "~> 0.34"},
      {:telemetry, "~> 1.3"},
      {:telemetry_metrics, "~> 1.0"}
    ]
  end
end
