defmodule Scripguessr.GamesTest do
  use Scripguessr.DataCase
  alias Scripguessr.{Games, Repo}
  alias Scripguessr.Games.Game

  test "persists prepared verses and progress independently of upstream availability" do
    assert {:ok, game} = Games.create("owner")
    Scripguessr.Corpus.Fake.reset(:down)
    Scripguessr.Corpus.Cache.clear()
    round = hd(game.snapshot["rounds"])

    assert {:ok, guessed} =
             Games.submit(game.id, "owner", 0, round["book"], to_string(round["chapter"]))

    assert {:ok, repeated} = Games.submit(game.id, "owner", 0, "2 Nephi", "33")
    assert repeated.snapshot == guessed.snapshot
    assert {:ok, _} = Games.advance(game.id, "owner", 0)
    assert {:ok, resumed} = Games.fetch(game.id, "owner")
    assert resumed.snapshot["rounds"] == guessed.snapshot["rounds"]
    assert resumed.snapshot["current"] == 1
    assert {:error, :unavailable} = Games.create("owner")
    assert Repo.aggregate(Game, :count) == 1
  end

  test "another browser cannot read, guess, or advance a game" do
    {:ok, game} = Games.create("owner")
    assert {:error, :not_found} = Games.fetch(game.id, "stranger")
    assert {:error, :not_found} = Games.submit(game.id, "stranger", 0, "1 Nephi", "1")
    assert {:error, :not_found} = Games.advance(game.id, "stranger", 0)
    assert {:error, :not_found} = Games.fetch("invalid", "owner")
  end
end
