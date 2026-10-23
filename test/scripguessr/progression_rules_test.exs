defmodule Scripguessr.Progression.RulesTest do
  use ExUnit.Case, async: true
  alias Scripguessr.Progression.Rules

  defp snapshot(points),
    do: %{"finished" => true, "rounds" => List.duplicate(%{"guess" => %{"points" => points}}, 5)}

  test "completion rewards participation with an accuracy bonus" do
    assert Rules.reward(snapshot(0)) == %{completion_xp: 100, accuracy_xp: 0}
    assert Rules.reward(snapshot(500)) == %{completion_xp: 100, accuracy_xp: 50}
    assert Rules.reward(snapshot(1_000)) == %{completion_xp: 100, accuracy_xp: 100}
    assert snapshot(1_000) |> Rules.reward() |> Rules.total_reward() == 200
  end

  test "unfinished, partial and invalid games earn no reward" do
    refute Rules.reward(nil)
    refute Rules.reward(Map.put(snapshot(500), "finished", false))
    refute Rules.reward(%{"finished" => true, "rounds" => []})
    refute Rules.reward(%{"finished" => true, "rounds" => List.duplicate(%{}, 5)})
    refute Rules.reward(snapshot(-1))
    refute Rules.reward(snapshot(1_001))
  end

  test "level progress advances exactly at each threshold" do
    for {xp, level, earned, needed} <- [
          {0, 1, 0, 500},
          {499, 1, 499, 500},
          {500, 2, 0, 1_000},
          {1_499, 2, 999, 1_000},
          {1_500, 3, 0, 1_500},
          {2_999, 3, 1_499, 1_500},
          {3_000, 4, 0, 2_000}
        ] do
      progress = Rules.progress(xp)
      assert progress.level == level
      assert progress.earned == earned
      assert progress.needed == needed
      assert progress.remaining == needed - earned
      assert progress.xp == xp
      assert progress.percent >= 0 and progress.percent < 100
    end

    assert Enum.map(1..4, &Rules.threshold/1) == [0, 500, 1_500, 3_000]
  end
end
