# Path: umbrella/apps/game_content/mix.exs
# Summary: game_content アプリ（ゲーム別コンテンツ: VampireSurvivor / MiniShooter）
defmodule GameContent.MixProject do
  use Mix.Project

  def project do
    [
      app: :game_content,
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

  defp deps do
    [
      {:game_engine, in_umbrella: true}
    ]
  end
end
