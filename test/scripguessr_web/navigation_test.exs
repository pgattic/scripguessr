defmodule ScripguessrWeb.NavigationTest do
  use Scripguessr.DataCase
  import Phoenix.ConnTest
  import Phoenix.LiveViewTest
  import Scripguessr.AccountFixtures
  import Scripguessr.GameFixtures
  alias Scripguessr.Games
  @endpoint ScripguessrWeb.Endpoint

  defp browser(owner, token \\ nil),
    do: build_conn() |> Plug.Test.init_test_session(%{"owner" => owner, "account_token" => token})

  test "primary navigation stays focused and personal pages share account navigation" do
    {_, token} = account("Reader")
    {:ok, home, _} = live(browser("owner", token), "/")
    assert has_element?(home, ".site-nav a[href='/'][aria-current=page]", "Play")
    refute has_element?(home, ".site-nav a[href='/saved-verses']")
    assert has_element?(home, "#user-tag", "Reader")
    assert has_element?(home, "#user-tag", "Level 1")
    assert has_element?(home, "#user-tag", "0 XP")
    assert has_element?(home, "#account-menu a[href='/saved-verses']")

    assert has_element?(
             home,
             "#header-sign-out[action='/account/sign-out'] input[name='_csrf_token']"
           )

    {:ok, atlas, _} = live(browser("owner", token), "/atlas")
    render_async(atlas)
    assert has_element?(atlas, ".site-nav a[href='/atlas'][aria-current=page]")
    {:ok, saved, _} = live(browser("owner", token), "/saved-verses")
    assert has_element?(saved, ".personal-nav a[href='/saved-verses'][aria-current=page]")
    assert has_element?(saved, ".personal-nav a[href='/account']")

    assert get(browser("owner", token), "/account") |> html_response(200) =~
             "Overview &amp; games"

    {:ok, guest, _} = live(browser("guest"), "/")
    refute has_element?(guest, "#user-tag")
    assert has_element?(guest, ".header-sign-in[href='/account/sign-in']")
  end

  test "continue game follows ownership and disappears when a game is complete" do
    {user, token} = account("reader")
    {:ok, game} = Games.create("owner", user.id)
    {:ok, signed_in, _} = live(browser("another-device", token), "/")
    assert has_element?(signed_in, "#resume-game[href='/game/#{game.id}']")
    {:ok, signed_out, _} = live(browser("owner"), "/")
    refute has_element?(signed_out, "#resume-game")
    finish_game(game, "owner", user.id)
    {:ok, finished, _} = live(browser("owner", token), "/")
    refute has_element?(finished, "#resume-game")

    {:ok, anonymous} = Games.create("guest")
    {:ok, guest, _} = live(browser("guest"), "/")
    assert has_element?(guest, "#resume-game[href='/game/#{anonymous.id}']")
    assert has_element?(guest, "#start-game", "New game")
    {:ok, stranger, _} = live(browser("stranger"), "/")
    refute has_element?(stranger, "#resume-game")
  end

  test "play again starts directly from results and preserves results on an upstream failure" do
    {:ok, game} = Games.create("owner")
    finish_game(game, "owner")
    {:ok, view, _} = live(browser("owner"), "/game/#{game.id}")
    Scripguessr.Corpus.Fake.reset(:down)
    view |> element("#play-again") |> render_click()
    render_async(view)
    assert has_element?(view, ".summary-page [role=alert]", "couldn’t load")
    assert has_element?(view, ".final-score")
    Scripguessr.Corpus.Fake.reset()
    view |> element("#play-again") |> render_click()
    {path, _} = assert_redirect(view)
    assert path =~ "/game/"
    refute path == "/game/#{game.id}"
  end
end
