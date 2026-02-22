defmodule Game.LevelSystem do
  @moduledoc """
  Level-up management system.

  Weapon choice generation is pure Elixir data transformation:
  no side effects, easy to test, easy to extend.
  """

  @all_weapons [:magic_wand, :axe, :cross]

  @doc """
  Returns 3 weapon choices for the level-up screen.

  - weapon_levels: %{weapon_atom => level} (0 = not owned)
  - Prioritises unowned weapons first, then lowest-level weapons.
  - Weapons already at max level (Lv.8) are excluded.
  """
  def generate_weapon_choices(weapon_levels) when is_map(weapon_levels) do
    @all_weapons
    |> Enum.reject(fn w -> Map.get(weapon_levels, w, 0) >= 8 end)
    |> Enum.sort_by(fn w ->
      lv = Map.get(weapon_levels, w, 0)
      # 未所持 (0) を最優先、次に低レベル順
      if lv == 0, do: -1, else: lv
    end)
    |> Enum.take(3)
  end

  # 後方互換: 旧シグネチャ（リスト）でも動作するよう残す
  def generate_weapon_choices(current_weapons) when is_list(current_weapons) do
    not_owned = Enum.reject(@all_weapons, &(&1 in current_weapons))
    candidates =
      if length(not_owned) >= 3, do: not_owned, else: not_owned ++ @all_weapons
    candidates |> Enum.uniq() |> Enum.take(3)
  end

  @doc "Human-readable weapon name with level for logging."
  def weapon_label(weapon, level) when is_integer(level) and level > 1 do
    "#{weapon_label(weapon)} Lv.#{level}"
  end
  def weapon_label(weapon, _level), do: weapon_label(weapon)

  @doc "Human-readable weapon name for logging."
  def weapon_label(:magic_wand), do: "Magic Wand (auto-aim)"
  def weapon_label(:axe),        do: "Axe (upward throw)"
  def weapon_label(:cross),      do: "Cross (4-way fire)"
  def weapon_label(other),       do: to_string(other)
end
