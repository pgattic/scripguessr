defmodule Scripguessr.DailyGamesTest do
  use Scripguessr.DataCase
  import Scripguessr.AccountFixtures
  import Scripguessr.GameFixtures
  alias Scripguessr.{Games, Repo, Leaderboards, Progression}
  alias Scripguessr.Challenges.Challenge
  alias Scripguessr.Games.Game

  test "all players share frozen prompts but have independent, private attempts" do
    {alice, _} = account("alice")
    {bob, _} = account("bob")
    {:ok, first} = Games.start_daily("alice-browser", alice.id)
    {:ok, second} = Games.start_daily("bob-browser", bob.id)
    assert first.id != second.id
    assert first.challenge_id == second.challenge_id
    assert first.snapshot == second.snapshot
    round = hd(first.snapshot["rounds"])

    {:ok, _} =
      Games.submit(
        first.id,
        "another-browser",
        0,
        round["book"],
        to_string(round["chapter"]),
        alice.id
      )

    {:ok, resumed} = Games.start_daily("new-browser", alice.id)
    assert resumed.id == first.id
    assert hd(resumed.snapshot["rounds"])["guess"]
    {:ok, untouched} = Games.fetch(second.id, "bob-browser", bob.id)
    refute hd(untouched.snapshot["rounds"])["guess"]
    assert {:error, :not_found} = Games.fetch(first.id, "alice-browser")
    assert {:error, :not_found} = Games.fetch(first.id, "bob-browser", bob.id)
    assert {:error, :not_found} = Games.submit(first.id, "bob-browser", 0, "Omni", "1", bob.id)
    assert {:error, :account_required} = Games.start_daily("guest", nil)
    assert Games.unclaimed_count("alice-browser") == 0
  end

  test "concurrent starts choose one challenge and one attempt per account" do
    {user, _} = account("reader")

    games =
      1..6
      |> Task.async_stream(fn _ -> Games.start_daily("owner", user.id) end,
        max_concurrency: 6,
        timeout: 20_000
      )
      |> Enum.map(fn {:ok, {:ok, game}} -> game end)

    assert games |> Enum.map(& &1.id) |> Enum.uniq() |> length() == 1
    assert Repo.aggregate(Challenge, :count) == 1
    assert Repo.aggregate(Game, :count) == 1
  end

  test "saved daily challenges remain playable through upstream outages and award XP once" do
    {user, _} = account("reader")
    {other, _} = account("other")
    {:ok, game} = Games.start_daily("owner", user.id)
    Scripguessr.Corpus.Fake.reset(:down)
    Scripguessr.Corpus.Cache.clear()
    {:ok, same} = Games.start_daily("other-device", user.id)
    assert same.id == game.id
    {:ok, other_game} = Games.start_daily("other-owner", other.id)
    assert other_game.snapshot == game.snapshot
    assert Leaderboards.daily(Date.utc_today()) == []
    assert Leaderboards.daily_rank(Date.utc_today(), user.id) == nil
    finish_game(game, "owner", user.id)
    assert [%{score: 5000, rank: 1, username: "reader"}] = Leaderboards.daily(Date.utc_today())
    assert Progression.for_user(user.id).xp == 200
    {:ok, finished} = Games.start_daily("owner", user.id)
    assert finished.id == game.id
    assert finished.snapshot["finished"]
    assert {:error, :stale_round} = Games.advance(game.id, "owner", 4, user.id)
    assert Progression.for_user(user.id).xp == 200
    assert Games.claim("owner", other.id) == 0
    assert Leaderboards.daily_rank(Date.utc_today(), user.id).score == 5000
  end

  test "expired attempts reject submissions and advances, while the next day gets a fresh attempt" do
    {user, _} = account("reader")
    {:ok, game} = Games.start_daily("owner", user.id)
    round = hd(game.snapshot["rounds"])

    {:ok, _} =
      Games.submit(game.id, "owner", 0, round["book"], to_string(round["chapter"]), user.id)

    yesterday = Date.add(Date.utc_today(), -1)
    Repo.update!(Ecto.Changeset.change(game.challenge, day: yesterday))
    assert {:error, :daily_closed} = Games.advance(game.id, "owner", 0, user.id)

    assert {:error, :daily_closed} =
             Games.submit(
               game.id,
               "owner",
               0,
               round["book"],
               to_string(round["chapter"]),
               user.id
             )

    assert Leaderboards.daily(yesterday) == []
    assert Progression.for_user(user.id).xp == 0
    {:ok, fresh} = Games.start_daily("owner", user.id)
    refute fresh.id == game.id
    refute fresh.challenge_id == game.challenge_id
    assert Games.daily_for_user(yesterday, user.id).id == game.id
    assert Games.daily_for_user(Date.utc_today(), user.id).id == fresh.id
    assert Games.latest_unfinished("owner", user.id) == nil
    {:ok, random} = Games.create("owner", user.id)
    assert Games.latest_unfinished("owner", user.id).id == random.id
  end

  test "daily scores are independent of lifetime XP and equal scores share ranks" do
    {amy, _} = account("amy")
    {ben, _} = account("Ben")
    {zed, _} = account("zed")

    for user <- [ben, amy] do
      {:ok, game} = Games.start_daily("owner", user.id)
      finish_game(game, "owner", user.id)
    end

    {:ok, random} = Games.create("owner", zed.id)
    finish_game(random, "owner", zed.id)

    assert [%{username: "amy", score: 5000, rank: 1}, %{username: "Ben", score: 5000, rank: 1}] =
             Leaderboards.daily(Date.utc_today())

    assert Leaderboards.daily_rank(Date.utc_today(), ben.id).rank == 1
    assert Leaderboards.daily_rank(Date.utc_today(), zed.id) == nil
    assert Leaderboards.daily(Date.add(Date.utc_today(), -1)) == []
    assert length(Leaderboards.xp()) == 3
  end

  test "a failed initial preparation leaves no daily challenge or attempt" do
    {user, _} = account("reader")
    Scripguessr.Corpus.Fake.reset(:down)
    assert {:error, _} = Games.start_daily("owner", user.id)
    assert Repo.aggregate(Challenge, :count) == 0
    assert Repo.aggregate(Game, :count) == 0
    Scripguessr.Corpus.Fake.reset()
    assert {:ok, _} = Games.start_daily("owner", user.id)
  end

  test "daily top 50 is bounded while a player's rank covers the whole board, including zero scores" do
    challenge = Repo.insert!(%Challenge{day: Date.utc_today(), snapshot: %{}})

    users =
      for n <- 1..52 do
        username = "player#{n}"

        user =
          Repo.insert!(%Scripguessr.Accounts.User{
            username: username,
            username_normalized: username,
            hashed_password: "unused",
            recovery_hash: "unused"
          })

        Repo.insert!(%Game{
          user_id: user.id,
          owner_hash: "unused",
          challenge_id: challenge.id,
          snapshot: %{"finished" => true},
          score: 52 - n
        })

        user
      end

    assert length(Leaderboards.daily(Date.utc_today())) == 50
    assert %{rank: 52, score: 0} = Leaderboards.daily_rank(Date.utc_today(), List.last(users).id)
  end
end
