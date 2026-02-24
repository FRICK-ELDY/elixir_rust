defmodule Game.VampireSurvivor.SpawnSystemTest do
  @moduledoc """
  SpawnSystem の純粋関数の単体テスト。

  ※ `maybe_spawn/3` と `spawn_with_elites/3` は NIF 呼び出しを含むため、
  統合テスト向けとして未実装。純粋関数（`current_wave/1`, `wave_label/1`, `enemy_kind_for_wave/1`）のみをテスト対象とする。
  """
  use ExUnit.Case, async: true

  describe "current_wave/1" do
    test "0〜30秒は interval=3000, count=3 のチュートリアルウェーブ" do
      assert Game.VampireSurvivor.SpawnSystem.current_wave(0) == {3000, 3}
      assert Game.VampireSurvivor.SpawnSystem.current_wave(15) == {3000, 3}
      assert Game.VampireSurvivor.SpawnSystem.current_wave(29.9) == {3000, 3}
    end

    test "30〜60秒は interval=2000, count=5 のウォームアップ" do
      assert Game.VampireSurvivor.SpawnSystem.current_wave(30) == {2000, 5}
      assert Game.VampireSurvivor.SpawnSystem.current_wave(45) == {2000, 5}
      assert Game.VampireSurvivor.SpawnSystem.current_wave(59.9) == {2000, 5}
    end

    test "60〜120秒は interval=1500, count=8" do
      assert Game.VampireSurvivor.SpawnSystem.current_wave(60) == {1500, 8}
      assert Game.VampireSurvivor.SpawnSystem.current_wave(90) == {1500, 8}
    end

    test "120〜180秒は interval=1000, count=12" do
      assert Game.VampireSurvivor.SpawnSystem.current_wave(120) == {1000, 12}
      assert Game.VampireSurvivor.SpawnSystem.current_wave(150) == {1000, 12}
    end

    test "180秒〜は interval=800, count=15 の最終盤" do
      assert Game.VampireSurvivor.SpawnSystem.current_wave(180) == {800, 15}
      assert Game.VampireSurvivor.SpawnSystem.current_wave(300) == {800, 15}
      assert Game.VampireSurvivor.SpawnSystem.current_wave(999) == {800, 15}
    end

    test "負の経過時間はフォールバック" do
      # List.last with default when filter returns [] -> uses {0, 800, 20} default
      assert Game.VampireSurvivor.SpawnSystem.current_wave(-1) == {800, 20}
    end
  end

  describe "wave_label/1" do
    test "経過時間に応じたラベルを返す" do
      assert Game.VampireSurvivor.SpawnSystem.wave_label(0) == "Wave 1 - Tutorial"
      assert Game.VampireSurvivor.SpawnSystem.wave_label(29) == "Wave 1 - Tutorial"
      assert Game.VampireSurvivor.SpawnSystem.wave_label(30) == "Wave 2 - Warming Up (Bat added)"
      assert Game.VampireSurvivor.SpawnSystem.wave_label(60) == "Wave 3 - Skeleton added"
      assert Game.VampireSurvivor.SpawnSystem.wave_label(120) == "Wave 4 - Ghost added (wall-pass)"
      assert Game.VampireSurvivor.SpawnSystem.wave_label(180) == "Wave 5 - Golem added"
      assert Game.VampireSurvivor.SpawnSystem.wave_label(599) == "Wave 5 - Golem added"
      assert Game.VampireSurvivor.SpawnSystem.wave_label(600) == "Wave 6 - ELITE (HP x3)"
    end
  end

  describe "enemy_kind_for_wave/1" do
    test "0〜30秒は常に :slime" do
      for _ <- 1..20 do
        assert Game.VampireSurvivor.SpawnSystem.enemy_kind_for_wave(0) == :slime
        assert Game.VampireSurvivor.SpawnSystem.enemy_kind_for_wave(15) == :slime
        assert Game.VampireSurvivor.SpawnSystem.enemy_kind_for_wave(29) == :slime
      end
    end

    test "30〜60秒は :slime または :bat" do
      results = for _ <- 1..50, do: Game.VampireSurvivor.SpawnSystem.enemy_kind_for_wave(45)
      assert Enum.all?(results, &(&1 in [:slime, :bat]))
    end

    test "60〜120秒は :slime, :bat, :skeleton のいずれか" do
      results = for _ <- 1..50, do: Game.VampireSurvivor.SpawnSystem.enemy_kind_for_wave(90)
      assert Enum.all?(results, &(&1 in [:slime, :bat, :skeleton]))
    end

    test "120〜180秒は :slime, :bat, :skeleton, :ghost のいずれか" do
      results = for _ <- 1..50, do: Game.VampireSurvivor.SpawnSystem.enemy_kind_for_wave(150)
      assert Enum.all?(results, &(&1 in [:slime, :bat, :skeleton, :ghost]))
    end

    test "180秒〜は :slime, :bat, :skeleton, :ghost, :golem のいずれか" do
      results = for _ <- 1..50, do: Game.VampireSurvivor.SpawnSystem.enemy_kind_for_wave(200)
      assert Enum.all?(results, &(&1 in [:slime, :bat, :skeleton, :ghost, :golem]))
    end
  end
end
