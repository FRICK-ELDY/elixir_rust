defmodule Game.MixProject do
  use Mix.Project

  def project do
    [
      app: :game,
      version: "0.1.0",
      elixir: "~> 1.19",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
    ]
  end

  def application do
    [
      extra_applications: [:logger],
      mod: {Game.Application, []},
    ]
  end

  defp deps do
    [
      {:rustler, "~> 0.34"},
      {:telemetry, "~> 1.3"},
      {:telemetry_metrics, "~> 1.0"},
    ]
  end
end
