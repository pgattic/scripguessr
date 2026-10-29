defmodule Scripguessr.GameFixtures do
  alias Scripguessr.Games
  alias Scripguessr.{Repo, Corpus.Cache, Challenges.Challenge}

  def daily_game(user_id, day, finished? \\ true) do
    {:ok, prepared} = Cache.prepare()

    snapshot =
      if finished? do
        Enum.with_index(prepared["rounds"])
        |> Enum.reduce(prepared, fn {round, index}, snapshot ->
          {:ok, snapshot} =
            Games.Rules.submit(snapshot, index, round["book"], to_string(round["chapter"]))

          {:ok, snapshot} = Games.Rules.advance(snapshot, index)
          snapshot
        end)
      else
        prepared
      end

    challenge =
      Repo.get_by(Challenge, day: day) || Repo.insert!(%Challenge{day: day, snapshot: prepared})

    Repo.insert!(%Games.Game{
      user_id: user_id,
      owner_hash: :crypto.hash(:sha256, "owner"),
      challenge_id: challenge.id,
      snapshot: snapshot,
      score: if(finished?, do: Games.Rules.total(snapshot))
    })
  end

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
