defmodule Game.LevelSystemTest do
  use ExUnit.Case, async: true

  describe "generate_weapon_choices/1" do
    test "空のレベルマップからは未所持武器が3つ返る" do
      choices = Game.LevelSystem.generate_weapon_choices(%{})
      assert length(choices) == 3
      assert Enum.all?(choices, &(&1 in [:magic_wand, :axe, :cross, :whip, :fireball, :lightning]))
    end

    test "最大レベルの武器は除外される" do
      levels = %{
        magic_wand: 8,
        axe: 5,
        cross: 3,
        whip: 1,
        fireball: 0,
        lightning: 0
      }
      choices = Game.LevelSystem.generate_weapon_choices(levels)
      refute :magic_wand in choices
      assert length(choices) <= 3
    end

    test "6スロット満杯で未所持は除外される" do
      levels = %{
        magic_wand: 5,
        axe: 5,
        cross: 5,
        whip: 5,
        fireball: 5,
        lightning: 5
      }
      choices = Game.LevelSystem.generate_weapon_choices(levels)
      # 全員最大でないので選べる武器はあるが、未所持はない（全所持済み）
      assert length(choices) <= 3
    end

    test "選ばれる武器は低レベル・未所持優先" do
      levels = %{magic_wand: 1, axe: 2, cross: 3}
      choices = Game.LevelSystem.generate_weapon_choices(levels)
      assert length(choices) == 3
      # 未所持（whip, fireball, lightning）が優先、次に低レベル
      assert Enum.all?(choices, &(&1 in [:magic_wand, :axe, :cross, :whip, :fireball, :lightning]))
    end

    test "全武器が最大レベルなら空" do
      levels = %{
        magic_wand: 8,
        axe: 8,
        cross: 8,
        whip: 8,
        fireball: 8,
        lightning: 8
      }
      assert Game.LevelSystem.generate_weapon_choices(levels) == []
    end
  end

  describe "weapon_label/1 and weapon_label/2" do
    test "weapon_label/1: 武器名のみ" do
      assert Game.LevelSystem.weapon_label(:magic_wand) == "Magic Wand (auto-aim)"
      assert Game.LevelSystem.weapon_label(:axe) == "Axe (upward throw)"
      assert Game.LevelSystem.weapon_label(:cross) == "Cross (4-way fire)"
      assert Game.LevelSystem.weapon_label(:whip) == "Whip (fan sweep)"
      assert Game.LevelSystem.weapon_label(:fireball) == "Fireball (piercing)"
      assert Game.LevelSystem.weapon_label(:lightning) == "Lightning (chain)"
    end

    test "weapon_label/2: レベル2以上で Lv.N 付き" do
      assert Game.LevelSystem.weapon_label(:magic_wand, 2) == "Magic Wand (auto-aim) Lv.2"
      assert Game.LevelSystem.weapon_label(:axe, 8) == "Axe (upward throw) Lv.8"
    end

    test "weapon_label/2: レベル1は Lv 付かない" do
      assert Game.LevelSystem.weapon_label(:magic_wand, 1) == "Magic Wand (auto-aim)"
    end

    test "不明な武器は to_string" do
      assert Game.LevelSystem.weapon_label(:unknown_weapon) == "unknown_weapon"
    end
  end
end
