defmodule Game.BossSystem do
  @moduledoc """
  ボスエネミーの出現スケジュールを管理する純粋関数モジュール。

  Elixir の強み:
  - ボス出現タイミングはパターンマッチで宣言的に記述
  - 状態は不変マップ — 副作用なし、テスト容易
  - GameLoop の phase 遷移と連携して演出フェーズを制御

  ボス出現スケジュール:
    180s (3分)  : Slime King  — HP 1000、スライム召喚
    360s (6分)  : Bat Lord    — HP 2000、高速突進 + 無敵
    540s (9分)  : Stone Golem — HP 5000、岩投げ範囲攻撃
  """

  @boss_schedule [
    {180, :slime_king,   "Slime King"},
    {360, :bat_lord,     "Bat Lord"},
    {540, :stone_golem,  "Stone Golem"},
  ]

  # ボス警告フェーズの継続時間（ミリ秒）
  @boss_alert_duration_ms 3_000

  @doc """
  現在の経過時間（秒）と既出現ボスリストから、
  次に出現すべきボスを返す。

  戻り値:
    {:spawn, boss_atom, boss_name} — 出現タイミングに達した
    :no_boss                       — 出現条件を満たさない
  """
  def check_spawn(elapsed_sec, spawned_bosses) when is_list(spawned_bosses) do
    @boss_schedule
    |> Enum.find(fn {trigger_sec, kind, _name} ->
      elapsed_sec >= trigger_sec and kind not in spawned_bosses
    end)
    |> case do
      {_sec, kind, name} -> {:spawn, kind, name}
      nil                -> :no_boss
    end
  end

  @doc """
  ボス出現時の警告メッセージを返す。
  """
  def alert_message(boss_name) do
    "⚠️  BOSS INCOMING: #{boss_name}!"
  end

  @doc """
  ボス警告フェーズの継続時間（ミリ秒）を返す。
  """
  def alert_duration_ms, do: @boss_alert_duration_ms

  @doc """
  ボス種別のラベルを返す。
  """
  def boss_label(:slime_king),  do: "Slime King"
  def boss_label(:bat_lord),    do: "Bat Lord"
  def boss_label(:stone_golem), do: "Stone Golem"
  def boss_label(other),        do: to_string(other)
end
