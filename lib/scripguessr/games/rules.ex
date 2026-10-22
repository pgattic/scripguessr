defmodule Scripguessr.Games.Rules do
  @moduledoc "Pure rules operating on a frozen session snapshot."
  def submit(snapshot, index, book, chapter) do
    current = snapshot["current"]
    round = Enum.at(snapshot["rounds"], current)

    cond do
      snapshot["finished"] or index != current ->
        {:error, :stale_round}

      round["guess"] != nil ->
        {:ok, snapshot}

      true ->
        with {:ok, number} <- chapter_number(chapter),
             guess_index when is_integer(guess_index) <- chapter_index(snapshot, book, number),
             answer_index when is_integer(answer_index) <-
               chapter_index(snapshot, round["book"], round["chapter"]) do
          distance = abs(guess_index - answer_index)

          guess = %{
            "book" => book,
            "chapter" => number,
            "distance" => distance,
            "points" => max(0, 1000 - 22 * distance)
          }

          rounds = List.update_at(snapshot["rounds"], current, &Map.put(&1, "guess", guess))
          {:ok, Map.put(snapshot, "rounds", rounds)}
        else
          _ -> {:error, :invalid_guess}
        end
    end
  end

  def advance(snapshot, index) do
    current = snapshot["current"]

    cond do
      index != current or snapshot["finished"] -> {:error, :stale_round}
      is_nil(Enum.at(snapshot["rounds"], current)["guess"]) -> {:error, :guess_required}
      current == 4 -> {:ok, Map.put(snapshot, "finished", true)}
      true -> {:ok, Map.put(snapshot, "current", current + 1)}
    end
  end

  def total(snapshot),
    do:
      Enum.reduce(snapshot["rounds"], 0, fn r, sum ->
        sum + (get_in(r, ["guess", "points"]) || 0)
      end)

  defp chapter_index(snapshot, book, chapter),
    do: Enum.find_index(snapshot["chapters"], &(&1["book"] == book and &1["chapter"] == chapter))

  defp chapter_number(chapter) when is_binary(chapter) do
    case Integer.parse(chapter) do
      {n, ""} when n > 0 -> {:ok, n}
      _ -> :error
    end
  end

  defp chapter_number(_), do: :error
end
