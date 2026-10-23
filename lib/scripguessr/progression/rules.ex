defmodule Scripguessr.Progression.Rules do
  @moduledoc "XP rewards and level thresholds, independent of persistence."
  alias Scripguessr.Games.Rules, as: GameRules

  def reward(%{"finished" => true, "rounds" => rounds} = snapshot)
      when is_list(rounds) and length(rounds) == 5 do
    if Enum.all?(rounds, &answered?/1) do
      %{completion_xp: 100, accuracy_xp: div(GameRules.total(snapshot), 50)}
    end
  end

  def reward(_), do: nil

  def total_reward(%{completion_xp: completion, accuracy_xp: accuracy}), do: completion + accuracy

  def threshold(level) when is_integer(level) and level >= 1, do: 250 * level * (level - 1)

  def progress(xp) when is_integer(xp) and xp >= 0 do
    {level, earned, needed} = locate_level(xp, 1, 500)

    %{
      xp: xp,
      level: level,
      earned: earned,
      needed: needed,
      remaining: needed - earned,
      percent: 100 * earned / needed
    }
  end

  defp locate_level(xp, level, needed) when xp < needed, do: {level, xp, needed}
  defp locate_level(xp, level, needed), do: locate_level(xp - needed, level + 1, needed + 500)

  defp answered?(%{"guess" => %{"points" => points}}),
    do: is_integer(points) and points >= 0 and points <= 1_000

  defp answered?(_), do: false
end
