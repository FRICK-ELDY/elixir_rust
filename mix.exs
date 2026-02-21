defmodule Game.MixProject do
  use Mix.Project

  def project do
    [
      app: :game,
      version: "0.1.0",
      elixir: "~> 1.19",
      start_permanent: Mix.env() == :prod,
      build_path: build_path(),
      deps: deps(),
    ]
  end

  def application do
    [
      extra_applications: [:logger],
      mod: {Game.Application, []},
    ]
  end

  # デバッグ/リリースで出力先を切り替える
  defp build_path do
    case Mix.env() do
      :prod -> "platform/windows/_build/release"
      _     -> "platform/windows/_build/debug"
    end
  end

  defp deps do
    [
      {:rustler, "~> 0.34"},
    ]
  end
end
