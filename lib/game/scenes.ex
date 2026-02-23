defmodule Game.Scenes do
  @moduledoc """
  シーン модуルの名前空間。

  G2 シーン管理システムで使用するシーン:
  - `Game.Scenes.Playing`  — プレイ中（物理演算・スポーン・チェック）
  - `Game.Scenes.LevelUp`  — レベルアップ武器選択
  - `Game.Scenes.BossAlert` — ボス出現警告
  - `Game.Scenes.GameOver` — ゲームオーバー

  新規シーン（タイトル、ステージ選択、設定画面など）を追加する場合は、
  `Game.SceneBehaviour` を実装し、`Game.SceneManager.push_scene/2` で遷移する。
  """
end
