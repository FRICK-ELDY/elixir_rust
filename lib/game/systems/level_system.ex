defmodule Game.LevelSystem do
  @moduledoc """
  Level-up management system.

  Weapon choice generation is pure Elixir data transformation:
  no side effects, easy to test, easy to extend.
  """

  @doc """
  Returns 3 weapon choices for the level-up screen.
  Prioritises weapons the player does not yet own.
  """
  def generate_weapon_choices(current_weapons) do
    all_weapons = [:magic_wand, :axe, :cross]

    not_owned = Enum.reject(all_weapons, &(&1 in current_weapons))

    candidates =
      if length(not_owned) >= 3, do: not_owned, else: not_owned ++ all_weapons

    candidates |> Enum.uniq() |> Enum.take(3)
  end

  @doc "Human-readable weapon name for logging."
  def weapon_label(:magic_wand), do: "Magic Wand (auto-aim)"
  def weapon_label(:axe),        do: "Axe (upward throw)"
  def weapon_label(:cross),      do: "Cross (4-way fire)"
  def weapon_label(other),       do: to_string(other)
end
