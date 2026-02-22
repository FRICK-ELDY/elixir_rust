defmodule Game.LevelSystem do
  @moduledoc """
  Level-up management system.

  Weapon choice generation is pure Elixir data transformation:
  no side effects, easy to test, easy to extend.
  """

  @all_weapons [:magic_wand, :axe, :cross, :whip, :fireball, :lightning]
  # Must match MAX_WEAPON_LEVEL and MAX_WEAPON_SLOTS in native/game_native/src/weapon.rs
  @max_weapon_level 8
  @max_weapon_slots 6

  @doc """
  Returns up to 3 weapon choices for the level-up screen.

  Exclusion rules (must mirror Rust add_weapon logic):
  - Weapons at max level (Lv.#{@max_weapon_level}) are excluded.
  - Unowned weapons are excluded when all #{@max_weapon_slots} slots are already filled.

  Sort order: unowned first, then lowest-level first.
  """
  def generate_weapon_choices(weapon_levels) when is_map(weapon_levels) do
    slots_full? = map_size(weapon_levels) >= @max_weapon_slots

    @all_weapons
    |> Enum.reject(fn w ->
      lv = Map.get(weapon_levels, w, 0)
      # Exclude max-level weapons
      lv >= @max_weapon_level or
      # Exclude unowned weapons when slots are full (Rust would silently no-op)
      (slots_full? and lv == 0)
    end)
    |> Enum.sort_by(fn w ->
      lv = Map.get(weapon_levels, w, 0)
      if lv == 0, do: -1, else: lv
    end)
    |> Enum.take(3)
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
  def weapon_label(:whip),       do: "Whip (fan sweep)"
  def weapon_label(:fireball),   do: "Fireball (piercing)"
  def weapon_label(:lightning),  do: "Lightning (chain)"
  def weapon_label(other),       do: to_string(other)
end
