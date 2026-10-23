defmodule Scripguessr.ProgressionTest do
  use Scripguessr.DataCase
  import Scripguessr.AccountFixtures
  import Scripguessr.GameFixtures
  alias Scripguessr.{Games, Progression, Repo}
  alias Scripguessr.Progression.Reward

  test "only completing a game awards XP, and refreshes or repeated submissions cannot duplicate it" do
    {user, _} = account("reader")
    {:ok, game} = Games.create("owner", user.id)
    assert Progression.for_user(user.id).xp == 0

    for {round, index} <- Enum.with_index(game.snapshot["rounds"]) do
      {:ok, _} =
        Games.submit(game.id, "owner", index, round["book"], to_string(round["chapter"]), user.id)

      assert Progression.for_user(user.id).xp == 0
      if index < 4, do: Games.advance(game.id, "owner", index, user.id)
    end

    {:ok, finished} = Games.advance(game.id, "owner", 4, user.id)
    assert Progression.for_user(user.id).xp == 200
    assert Progression.for_game(game.id).accuracy_xp == 100
    assert {:error, :stale_round} = Games.advance(game.id, "owner", 4, user.id)
    assert {:error, :stale_round} = Games.submit(game.id, "owner", 4, "1 Nephi", "1", user.id)
    assert {:ok, _} = Games.fetch(game.id, "another-device", user.id)
    assert Progression.for_user(user.id).xp == 200
    assert Progression.for_user(user.id).xp == 200
    assert Repo.aggregate(Reward, :count) == 1
    assert finished.snapshot["finished"]
  end

  test "completed anonymous games earn XP once when claimed and stay private to the claiming account" do
    {user, _} = account("reader")
    {other, _} = account("other")
    {:ok, game} = Games.create("guest")
    finish_game(game, "guest")
    refute Progression.for_game(game.id)
    assert Progression.for_user(user.id).xp == 0
    assert Games.claim("guest", user.id) == 1
    assert Progression.for_user(user.id).xp == 200
    assert Games.claim("guest", user.id) == 0
    assert Games.claim("guest", other.id) == 0
    assert Progression.for_user(user.id).xp == 200
    assert Progression.for_user(other.id).xp == 0
    assert Progression.for_game(game.id).user_id == user.id
    assert Repo.aggregate(Reward, :count) == 1
  end

  test "an unfinished claimed game earns XP on completion and crosses level boundaries" do
    {user, _} = account("reader")
    {:ok, unfinished} = Games.create("guest")
    assert Games.claim("guest", user.id) == 1
    assert Progression.for_user(user.id).xp == 0
    {:ok, claimed} = Games.fetch(unfinished.id, "another-device", user.id)
    finish_game(claimed, "another-device", user.id)

    for _ <- 1..2 do
      {:ok, game} = Games.create("owner", user.id)
      finish_game(game, "owner", user.id)
    end

    progress = Progression.for_user(user.id)
    assert progress.xp == 600
    assert progress.level == 2
    assert progress.earned == 100
    assert progress.remaining == 900
  end

  test "reading progression does not create or repair rewards" do
    {user, _} = account("reader")
    {:ok, game} = Games.create("owner", user.id)
    finish_game(game, "owner", user.id)
    reward = Progression.for_game(game.id)
    Repo.delete!(reward)

    assert Progression.for_user(user.id).xp == 0
    refute Progression.for_game(game.id)
    assert Repo.aggregate(Reward, :count) == 0
  end
end
