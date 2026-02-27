# Path: umbrella/mix.exs
# Summary: Umbrella ルート mix.exs（apps/ 配下の各アプリを管理）
defmodule GameUmbrella.MixProject do
  use Mix.Project

  def project do
    [
      apps_path: "apps",
      version: "0.1.0",
      elixir: "~> 1.19",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  defp deps do
    []
  end
end
