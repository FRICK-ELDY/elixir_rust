defmodule Game.VampireSurvivor do
  @moduledoc """
  Step 35–36: ヴァンサバを Game 実装として分離し、Engine.Game behaviour を実装。

  起動時の初期シーン構成、物理演算対象シーン、シーン遷移で使用する
  モジュール参照を提供する。
  """
  @behaviour Engine.Game

  # ── Engine.Game callbacks ───────────────────────────────────────

  @impl Engine.Game
  def render_type, do: :playing

  @impl Engine.Game
  def initial_scenes do
    [
      %{module: Game.VampireSurvivor.Scenes.Playing, init_arg: %{}}
    ]
  end

  # Step 38: 敵・武器・ボスの ID マッピング（Rust の u8 ID に相当）
  @impl Engine.Game
  def entity_registry do
    %{
      enemies: %{slime: 0, bat: 1, golem: 2, ghost: 3},
      weapons: %{
        magic_wand: 0, axe: 1, cross: 2, whip: 3, fireball: 4, lightning: 5
      },
      bosses: %{slime_king: 0, bat_lord: 1, stone_golem: 2}
    }
  end

  @impl Engine.Game
  def physics_scenes do
    [Game.VampireSurvivor.Scenes.Playing]
  end

  @impl Engine.Game
  def title, do: "Vampire Survivor"

  @impl Engine.Game
  def version, do: "0.1.0"

  @impl Engine.Game
  def context_defaults, do: %{}

  # Step 39: ゲーム別アセットパス
  @impl Engine.Game
  def assets_path, do: "vampire_survivor"

  # ── Vampire Survivor 固有（シーン遷移等で GameLoop が参照）──

  @doc "レベルアップ武器選択シーンのモジュール"
  def level_up_scene, do: Game.VampireSurvivor.Scenes.LevelUp

  @doc "ボス出現警告シーンのモジュール（Step 41: pause_physics 用）"
  def boss_alert_scene, do: Game.VampireSurvivor.Scenes.BossAlert

  @doc "ゲームオーバーシーンのモジュール"
  def game_over_scene, do: Game.VampireSurvivor.Scenes.GameOver

  @doc "StressMonitor / GameLoop のログ用ウェーブラベル"
  def wave_label(elapsed_sec), do: Game.VampireSurvivor.SpawnSystem.wave_label(elapsed_sec)

  @doc "武器の表示用ラベル（ログ用）"
  def weapon_label(weapon, level), do: Game.VampireSurvivor.LevelSystem.weapon_label(weapon, level)
end
