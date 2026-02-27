# Path: umbrella/apps/game_network/mix.exs
# Summary: game_network アプリ（Phoenix Socket/Channel・認証・Presence）
defmodule GameNetwork.MixProject do
  use Mix.Project

  def project do
    [
      app: :game_network,
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
      mod: {GameNetwork.Application, []}
    ]
  end

  defp deps do
    [
      {:game_engine, in_umbrella: true},
      {:phoenix, "~> 1.7"},
      {:phoenix_pubsub, "~> 2.1"},
      {:ecto_sql, "~> 3.10"},
      {:postgrex, ">= 0.0.0"}
    ]
  end
end
