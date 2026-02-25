# Path: test/games/vampire_survivor/boss_system_test.exs
# Summary: BossSystem の単体テスト
defmodule Game.VampireSurvivor.BossSystemTest do
  use ExUnit.Case, async: true

  describe "check_spawn/2" do
    test "180秒で Slime King 出現" do
      assert Game.VampireSurvivor.BossSystem.check_spawn(180, []) == {:spawn, :slime_king, "Slime King"}
      assert Game.VampireSurvivor.BossSystem.check_spawn(181, []) == {:spawn, :slime_king, "Slime King"}
    end

    test "360秒で Bat Lord 出現（Slime King 未出現時）" do
      assert Game.VampireSurvivor.BossSystem.check_spawn(360, []) == {:spawn, :slime_king, "Slime King"}
    end

    test "360秒で Bat Lord 出現（Slime King 出現済み）" do
      assert Game.VampireSurvivor.BossSystem.check_spawn(360, [:slime_king]) == {:spawn, :bat_lord, "Bat Lord"}
    end

    test "540秒で Stone Golem 出現（前二者出現済み）" do
      assert Game.VampireSurvivor.BossSystem.check_spawn(540, [:slime_king, :bat_lord]) == {:spawn, :stone_golem, "Stone Golem"}
    end

    test "全ボス出現済みなら :no_boss" do
      assert Game.VampireSurvivor.BossSystem.check_spawn(600, [:slime_king, :bat_lord, :stone_golem]) == :no_boss
    end

    test "経過時間が足りないと :no_boss" do
      assert Game.VampireSurvivor.BossSystem.check_spawn(0, []) == :no_boss
      assert Game.VampireSurvivor.BossSystem.check_spawn(179, []) == :no_boss
    end

    test "同じボスは二度出現しない（出現済みをスキップして次を返す）" do
      # 540秒時点で slime_king のみ出現済み → bat_lord を返す（slime_king はスキップ）
      assert Game.VampireSurvivor.BossSystem.check_spawn(540, [:slime_king]) == {:spawn, :bat_lord, "Bat Lord"}
    end
  end

  describe "alert_message/1" do
    test "ボス名を含む警告メッセージ" do
      assert Game.VampireSurvivor.BossSystem.alert_message("Slime King") == "⚠️  BOSS INCOMING: Slime King!"
      assert Game.VampireSurvivor.BossSystem.alert_message("Bat Lord") == "⚠️  BOSS INCOMING: Bat Lord!"
    end
  end

  describe "alert_duration_ms/0" do
    test "3000ミリ秒を返す" do
      assert Game.VampireSurvivor.BossSystem.alert_duration_ms() == 3_000
    end
  end

  describe "boss_label/1" do
    test "ボス種別のラベル" do
      assert Game.VampireSurvivor.BossSystem.boss_label(:slime_king) == "Slime King"
      assert Game.VampireSurvivor.BossSystem.boss_label(:bat_lord) == "Bat Lord"
      assert Game.VampireSurvivor.BossSystem.boss_label(:stone_golem) == "Stone Golem"
    end

    test "不明なボスは to_string" do
      assert Game.VampireSurvivor.BossSystem.boss_label(:unknown) == "unknown"
    end
  end
end
