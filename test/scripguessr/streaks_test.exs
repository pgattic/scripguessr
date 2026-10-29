defmodule Scripguessr.StreaksTest do
  use Scripguessr.DataCase
  import Scripguessr.AccountFixtures
  import Scripguessr.GameFixtures
  alias Scripguessr.{Games, Streaks}

  test "new accounts and guests have no streak" do
    {user, _} = account("reader")
    assert Streaks.for_user(nil) == %{current: 0, best: 0, completed_today: false}
    assert Streaks.for_user(user.id) == Streaks.for_user(nil)
  end

  test "existing daily history supplies current and best runs across calendar boundaries" do
    {user, _} = account("reader")
    today = ~D[2028-03-01]

    for offset <- Enum.to_list(-14..-10) ++ Enum.to_list(-2..0) ++ [1],
        do: daily_game(user.id, Date.add(today, offset))

    assert Streaks.for_user(user.id, today) == %{current: 3, best: 5, completed_today: true}
  end

  test "yesterday's streak stays active until a full day is missed" do
    {user, _} = account("reader")
    today = ~D[2026-10-29]
    for offset <- -3..-1, do: daily_game(user.id, Date.add(today, offset))
    assert Streaks.for_user(user.id, today) == %{current: 3, best: 3, completed_today: false}

    assert Streaks.for_user(user.id, Date.add(today, 1)) == %{
             current: 0,
             best: 3,
             completed_today: false
           }

    daily_game(user.id, today)
    assert Streaks.for_user(user.id, Date.add(today, 1)).current == 4
  end

  test "unfinished dailies, random games, and other accounts cannot bridge a gap" do
    {user, _} = account("reader")
    {other, _} = account("other")
    today = Date.utc_today()
    daily_game(user.id, Date.add(today, -2))
    daily_game(user.id, Date.add(today, -1), false)
    daily_game(other.id, Date.add(today, -1))
    daily_game(user.id, today)
    {:ok, random} = Games.create("owner", user.id)
    finish_game(random, "owner", user.id)
    assert Streaks.for_user(user.id) == %{current: 1, best: 1, completed_today: true}
  end

  test "a zero-score daily extends the streak only after the final round is completed" do
    {user, _} = account("reader")
    daily_game(user.id, Date.add(Date.utc_today(), -1))
    {:ok, game} = Games.start_daily("owner", user.id)

    for index <- 0..4 do
      assert {:ok, _} = Games.submit(game.id, "owner", index, "Omni", "1", user.id)
      if index < 4, do: Games.advance(game.id, "owner", index, user.id)
    end

    assert Streaks.for_user(user.id) == %{current: 1, best: 1, completed_today: false}
    assert {:ok, %{score: 0}} = Games.advance(game.id, "owner", 4, user.id)
    assert Streaks.for_user(user.id) == %{current: 2, best: 2, completed_today: true}
    assert {:error, :stale_round} = Games.advance(game.id, "owner", 4, user.id)
    assert Streaks.for_user(user.id).current == 2
  end
end
