defmodule ScripguessrWeb.SavedVersesTest do
  use Scripguessr.DataCase
  import Phoenix.ConnTest
  import Phoenix.LiveViewTest
  import Scripguessr.AccountFixtures
  alias Scripguessr.{Games, SavedVerses}
  @endpoint ScripguessrWeb.Endpoint

  defp browser(token \\ nil),
    do:
      build_conn() |> Plug.Test.init_test_session(%{"owner" => "owner", "account_token" => token})

  test "saves are idempotent, private, ordered, and scoped on removal" do
    {user, _} = account("reader")
    {other, _} = account("other")
    verse = %{"book" => "1 Nephi", "chapter" => 1, "verse" => 1, "text" => "First text"}
    assert {:ok, _} = SavedVerses.save(user.id, verse, "fixture-v1")
    assert {:ok, _} = SavedVerses.save(user.id, verse, "fixture-v2")
    assert [first] = SavedVerses.list(user.id)
    assert first.text == "First text"
    assert SavedVerses.list(other.id) == []
    SavedVerses.remove_by_id(other.id, first.id)
    assert [_] = SavedVerses.list(user.id)
    SavedVerses.save(user.id, %{verse | "verse" => 2}, "fixture-v1")
    assert Enum.map(SavedVerses.list(user.id), & &1.verse) == [2, 1]
    SavedVerses.remove_by_id(user.id, first.id)
    assert [%{verse: 2}] = SavedVerses.list(user.id)
  end

  test "answer reveals save server text and readers share saved state" do
    {user, token} = account("reader")
    {:ok, game} = Games.create("owner", user.id)
    {:ok, view, _} = live(browser(token), "/game/#{game.id}")
    refute has_element?(view, "#answer-reveal .save-verse")
    render_click(view, "save-verse", %{"source" => "round", "index" => "0"})
    assert SavedVerses.list(user.id) == []
    Games.submit(game.id, "owner", 0, "1 Nephi", "1", user.id)
    {:ok, view, _} = live(browser(token), "/game/#{game.id}")
    view |> element("#answer-reveal .save-verse") |> render_click()
    assert has_element?(view, "#answer-reveal .save-verse[aria-pressed=true]", "Saved")
    assert [saved] = SavedVerses.list(user.id)
    assert saved.text == hd(game.snapshot["rounds"])["text"]
    view |> element("#read-chapter") |> render_click()
    render_async(view)
    assert has_element?(view, "#game-reader [data-verse='1'] .save-verse[aria-pressed=true]")
    view |> element("#game-reader [data-verse='1'] .save-verse") |> render_click()
    assert SavedVerses.list(user.id) == []
    assert has_element?(view, "#answer-reveal .save-verse[aria-pressed=false]")
  end

  test "saved page reads without the API, links to chapters, and removes entries" do
    {user, token} = account("reader")

    {:ok, verse} =
      SavedVerses.save(
        user.id,
        %{"book" => "1 Nephi", "chapter" => 1, "verse" => 1, "text" => "A saved passage."},
        "fixture-v1"
      )

    Scripguessr.Corpus.Fake.reset(:down)
    {:ok, view, _} = live(browser(token), "/saved-verses")
    assert has_element?(view, "#saved-#{verse.id}", "A saved passage.")
    assert has_element?(view, "a[href*='book=1+Nephi'][href*='chapter=1']", "1 Nephi 1:1")
    view |> element("#saved-#{verse.id} button") |> render_click()
    assert render(view) =~ "No saved verses yet"
    assert {:error, {:redirect, %{to: "/account/sign-in"}}} = live(browser(), "/saved-verses")
  end

  test "Atlas reader saves a verse without a game" do
    Scripguessr.Atlas.Cache.clear()
    {user, token} = account("reader")
    {:ok, view, _} = live(browser(token), "/atlas?book=1+Nephi&chapter=1")
    render_async(view)
    render_async(view)
    view |> element("#atlas-reader [data-verse='3'] .save-verse") |> render_click()

    assert [%{book: "1 Nephi", chapter: 1, verse: 3, text: "Invented study passage 3."}] =
             SavedVerses.list(user.id)

    assert has_element?(view, "#atlas-reader [data-verse='3'] .save-verse[aria-pressed=true]")
  end

  test "anonymous saves and revoked sessions cannot write" do
    {:ok, game} = Games.create("owner")
    Games.submit(game.id, "owner", 0, "1 Nephi", "1")
    {:ok, view, _} = live(browser(), "/game/#{game.id}")
    assert has_element?(view, "#answer-reveal a.save-verse", "Sign in to save")
    render_click(view, "save-verse", %{"source" => "round", "index" => "0"})
    {user, token} = account("reader")
    {:ok, view, _} = live(browser(token), "/game/#{game.id}")
    Scripguessr.Accounts.revoke_session(token)
    render_click(view, "save-verse", %{"source" => "round", "index" => "0"})
    assert_redirect(view, "/account/sign-in")
    assert SavedVerses.list(user.id) == []
  end
end
