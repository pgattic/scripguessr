defmodule Scripguessr.GameFixtures do
  alias Scripguessr.Games

  def finish_game(game, owner, user_id \\ nil) do
    Enum.with_index(game.snapshot["rounds"])
    |> Enum.reduce(game, fn {round, index}, game ->
      {:ok, _} =
        Games.submit(game.id, owner, index, round["book"], to_string(round["chapter"]), user_id)

      {:ok, game} = Games.advance(game.id, owner, index, user_id)
      game
    end)
  end
end
