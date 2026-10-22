defmodule Scripguessr.Games.RulesTest do
  use ExUnit.Case, async: true
  alias Scripguessr.Games.Rules

  defp snapshot do
    chapters =
      for {book, count} <- [{"1 Nephi", 22}, {"2 Nephi", 33}],
          n <- 1..count,
          do: %{"book" => book, "chapter" => n}

    round = %{
      "book" => "1 Nephi",
      "chapter" => 22,
      "verse" => 1,
      "text" => "Invented test passage."
    }

    %{
      "current" => 0,
      "finished" => false,
      "chapters" => chapters,
      "rounds" => List.duplicate(round, 5)
    }
  end

  test "scores exact, nearby, cross-book and distant chapters" do
    for {book, chapter, points} <- [
          {"1 Nephi", "22", 1000},
          {"1 Nephi", "21", 978},
          {"2 Nephi", "1", 978},
          {"2 Nephi", "33", 274}
        ] do
      assert {:ok, state} = Rules.submit(snapshot(), 0, book, chapter)
      assert Rules.total(state) == points
    end

    state = put_in(snapshot(), ["rounds", Access.at(0), "chapter"], 1)
    assert {:ok, state} = Rules.submit(state, 0, "2 Nephi", "33")
    assert Rules.total(state) == 0
  end

  test "validates guesses and requires a submitted guess before advancing" do
    assert {:error, :invalid_guess} = Rules.submit(snapshot(), 0, "Unknown", "1")
    assert {:error, :invalid_guess} = Rules.submit(snapshot(), 0, "1 Nephi", "0")
    assert {:error, :invalid_guess} = Rules.submit(snapshot(), 0, "1 Nephi", "1junk")
    assert {:error, :guess_required} = Rules.advance(snapshot(), 0)
  end

  test "repeated and stale requests cannot overwrite a guess or skip a round" do
    assert {:ok, state} = Rules.submit(snapshot(), 0, "1 Nephi", "22")
    assert {:ok, ^state} = Rules.submit(state, 0, "2 Nephi", "1")
    assert {:ok, state} = Rules.advance(state, 0)
    assert {:error, :stale_round} = Rules.advance(state, 0)
    assert {:error, :stale_round} = Rules.submit(state, 0, "1 Nephi", "1")
  end

  test "finishes only after all five guesses and totals accepted scores" do
    state =
      Enum.reduce(0..4, snapshot(), fn n, state ->
        {:ok, state} = Rules.submit(state, n, "1 Nephi", "22")
        {:ok, state} = Rules.advance(state, n)
        state
      end)

    assert state["finished"]
    assert Rules.total(state) == 5000
    assert {:error, :stale_round} = Rules.advance(state, 4)
  end
end
