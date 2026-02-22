defmodule Game.LevelSystem do
  @moduledoc """
  レベルアップ管理システム（Step 14）。
  経験値テーブルに基づいてレベルアップを判定し、
  武器選択肢（3 択）を生成する。
  """

  # インデックス = レベル（1 始まり）、値 = そのレベルに必要な累積 EXP
  @exp_table [0, 10, 25, 45, 70, 100, 135, 175, 220, 270]

  @doc """
  現在の経験値とレベルからレベルアップを判定する。
  Rust 側の `level_up_pending` フラグを確認して呼び出す。
  """
  def check_level_up(current_exp, current_level) do
    required = Enum.at(@exp_table, current_level, :infinity)
    if current_exp >= required do
      {:level_up, current_level + 1}
    else
      :no_change
    end
  end

  @doc """
  レベルアップ時に提示する武器選択肢を 3 つ返す。
  現在の武器リストを考慮して重複しにくい選択肢を生成する。
  """
  def generate_weapon_choices(current_weapons) do
    all_weapons = [:magic_wand, :axe, :cross]

    # 未所持の武器を優先、足りない場合は既存武器も含める
    not_owned = Enum.reject(all_weapons, &(&1 in current_weapons))

    candidates =
      if length(not_owned) >= 3 do
        not_owned
      else
        not_owned ++ all_weapons
      end

    candidates |> Enum.uniq() |> Enum.take(3)
  end

  @doc """
  武器アトムを人間が読みやすい文字列に変換する。
  """
  def weapon_label(:magic_wand), do: "Magic Wand（最近接敵に弾丸発射）"
  def weapon_label(:axe),        do: "Axe（上方向に斧を投擲）"
  def weapon_label(:cross),      do: "Cross（上下左右 4 方向に発射）"
  def weapon_label(other),       do: to_string(other)
end
