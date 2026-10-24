defmodule ScripguessrWeb.LeaderboardLiveTest do
  use Scripguessr.DataCase
  import Phoenix.ConnTest
  import Phoenix.LiveViewTest
  import Scripguessr.AccountFixtures
  import Scripguessr.GameFixtures
  alias Scripguessr.Games
  @endpoint ScripguessrWeb.Endpoint

  defp browser(token \\ nil),
    do:
      build_conn() |> Plug.Test.init_test_session(%{"owner" => "owner", "account_token" => token})

  test "guests can view the board and its empty state from the main navigation" do
    {:ok, view, _} = live(browser(), "/leaderboard")
    assert has_element?(view, ".site-nav a[href='/leaderboard'][aria-current=page]")
    refute has_element?(view, ".site-nav a[href='/'][aria-current]")
    assert has_element?(view, "#leaderboard-empty", "The first spot is open.")
    assert has_element?(view, "#leaderboard-empty a[href='/account/register']")
    refute has_element?(view, "#leaderboard")
  end

  test "public ranks show XP and levels, and mark the signed-in player's row" do
    {user, token} = account("reader")

    for _ <- 1..5 do
      {:ok, game} = Games.create("owner", user.id)
      finish_game(game, "owner", user.id)
    end

    {:ok, guest, _} = live(browser(), "/leaderboard")
    assert has_element?(guest, "#leaderboard tbody th[scope=row]", "reader")
    assert has_element?(guest, "#leaderboard .leaderboard-xp", "1,000")
    assert has_element?(guest, "#leaderboard tbody td:nth-child(3)", "2")
    refute has_element?(guest, ".leaderboard-you")
    refute has_element?(guest, "a[href^='/game/']")

    {:ok, signed_in, _} = live(browser(token), "/leaderboard")
    assert has_element?(signed_in, "#leaderboard .leaderboard-self", "reader")
    assert has_element?(signed_in, ".leaderboard-self .leaderboard-you", "You")
    assert has_element?(signed_in, "#user-tag", "1000 XP")
  end
end
