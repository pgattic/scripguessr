defmodule ScripguessrWeb.ProgressionTest do
  use Scripguessr.DataCase
  import Phoenix.ConnTest
  import Phoenix.LiveViewTest
  import Scripguessr.AccountFixtures
  import Scripguessr.GameFixtures
  alias Scripguessr.{Games, Progression}
  @endpoint ScripguessrWeb.Endpoint

  defp browser(owner, token \\ nil),
    do: build_conn() |> Plug.Test.init_test_session(%{"owner" => owner, "account_token" => token})

  test "finishing a game shows its XP and updated level, and refresh preserves the award" do
    {user, token} = account("reader")

    for _ <- 1..2 do
      {:ok, previous} = Games.create("owner", user.id)
      finish_game(previous, "owner", user.id)
    end

    {:ok, game} = Games.create("owner", user.id)
    {:ok, view, _} = live(browser("owner", token), "/game/#{game.id}")
    refute has_element?(view, "#game-xp-reward")

    for round <- game.snapshot["rounds"] do
      view |> element("#book-grid button", round["book"]) |> render_click()

      view
      |> element("#chapter-grid button[phx-value-chapter='#{round["chapter"]}']")
      |> render_click()

      view |> form("#guess-form") |> render_submit()
      view |> element("#next-round") |> render_click()
    end

    assert has_element?(view, "#game-xp-reward", "+200 XP")
    assert has_element?(view, "#game-xp-reward", "100 completion · 100 accuracy")
    assert has_element?(view, "#game-xp-progress", "Level 2")
    assert has_element?(view, "#game-xp-progress", "600 XP total")
    assert has_element?(view, "#game-xp-progress", "900 XP to level 3")

    assert has_element?(
             view,
             "#game-xp-progress [role=progressbar][aria-valuenow='100'][aria-valuemax='1000']"
           )

    {:ok, refreshed, _} = live(browser("new-device", token), "/game/#{game.id}")
    assert has_element?(refreshed, "#game-xp-reward", "+200 XP")
    assert Progression.for_user(user.id).xp == 600
    account_page = get(browser("new-device", token), "/account") |> html_response(200)
    assert account_page =~ "600 XP total"
    assert account_page =~ "Level 2"
  end

  test "anonymous results offer XP to save, and attaching the game turns it into earned XP" do
    {:ok, game} = Games.create("guest")
    finish_game(game, "guest")
    {:ok, view, _} = live(browser("guest"), "/game/#{game.id}")
    assert has_element?(view, "#game-xp-reward", "200 XP available")
    assert has_element?(view, "#game-xp-reward a[href='/account/register']")
    refute has_element?(view, "#game-xp-progress")
    {user, token} = account("reader")
    {:ok, signed_in, _} = live(browser("guest", token), "/game/#{game.id}")
    assert has_element?(signed_in, "#game-xp-reward a[href='/account']")
    assert Progression.for_user(user.id).xp == 0
    assert post(browser("guest", token), "/account/claim-games") |> redirected_to() == "/account"
    {:ok, saved, _} = live(browser("new-device", token), "/game/#{game.id}")
    assert has_element?(saved, "#game-xp-reward", "+200 XP")
    refute has_element?(saved, "#game-xp-reward", "available")
    assert has_element?(saved, "#game-xp-progress", "200 XP total")
  end
end
