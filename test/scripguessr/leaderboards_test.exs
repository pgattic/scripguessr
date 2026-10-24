defmodule Scripguessr.LeaderboardsTest do
  use Scripguessr.DataCase
  import Scripguessr.AccountFixtures
  import Scripguessr.GameFixtures
  alias Scripguessr.{Games, Leaderboards, Repo}
  alias Scripguessr.Accounts.User
  alias Scripguessr.Games.Game
  alias Scripguessr.Progression.Reward

  test "ranks total earned XP, shares ties, and uses the existing level rules" do
    {zoe, _} = account("Zoe")
    {amy, _} = account("amy")
    {ben, _} = account("Ben")
    {newcomer, _} = account("newcomer")

    for user <- [zoe, zoe, amy, amy, ben, ben, ben] do
      {:ok, game} = Games.create("owner", user.id)
      finish_game(game, "owner", user.id)
    end

    {:ok, _} = Games.create("owner", newcomer.id)
    {:ok, guest} = Games.create("guest")
    finish_game(guest, "guest")

    assert [
             %{username: "Ben", rank: 1, xp: 600, level: 2},
             %{username: "amy", rank: 2, xp: 400, level: 1},
             %{username: "Zoe", rank: 2, xp: 400, level: 1}
           ] = Leaderboards.xp()

    assert Games.claim("guest", newcomer.id) == 1
    assert [_, _, _, %{username: "newcomer", rank: 4, xp: 200}] = Leaderboards.xp()
  end

  test "bounds the board to 50 with deterministic ordering across ties" do
    for n <- 1..52 do
      username = "player" <> String.pad_leading(to_string(n), 2, "0")

      user =
        Repo.insert!(%User{
          username: username,
          username_normalized: username,
          hashed_password: "unused",
          recovery_hash: "unused"
        })

      game = Repo.insert!(%Game{user_id: user.id, owner_hash: "unused", snapshot: %{}})

      Repo.insert!(%Reward{
        game_id: game.id,
        user_id: user.id,
        completion_xp: 100,
        accuracy_xp: 0
      })
    end

    players = Leaderboards.xp()
    assert length(players) == 50
    assert hd(players).username == "player01"
    assert List.last(players).username == "player50"
    assert Enum.all?(players, &(&1.rank == 1))
  end
end
