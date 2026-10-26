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
    assert [first] = SavedVerses.page(user.id).entries
    assert first.text == "First text"
    assert SavedVerses.page(other.id).entries == []
    SavedVerses.remove_by_id(other.id, first.id)
    assert [_] = SavedVerses.page(user.id).entries
    SavedVerses.save(user.id, %{verse | "verse" => 2}, "fixture-v1")
    assert Enum.map(SavedVerses.page(user.id).entries, & &1.verse) == [2, 1]
    SavedVerses.remove_by_id(user.id, first.id)
    assert [%{verse: 2}] = SavedVerses.page(user.id).entries
  end

  test "answer reveals save server text and readers share saved state" do
    {user, token} = account("reader")
    {:ok, game} = Games.create("owner", user.id)
    {:ok, view, _} = live(browser(token), "/game/#{game.id}")
    refute has_element?(view, "#answer-reveal .save-verse")
    render_click(view, "save-verse", %{"source" => "round", "index" => "0"})
    assert SavedVerses.page(user.id).entries == []
    Games.submit(game.id, "owner", 0, "1 Nephi", "1", user.id)
    {:ok, view, _} = live(browser(token), "/game/#{game.id}")
    view |> element("#answer-reveal .save-verse") |> render_click()

    assert has_element?(
             view,
             "#answer-reveal .save-verse[aria-pressed=true][aria-label='Unsave verse']"
           )

    assert [saved] = SavedVerses.page(user.id).entries
    assert saved.text == hd(game.snapshot["rounds"])["text"]
    view |> element("#read-chapter") |> render_click()
    render_async(view)
    assert has_element?(view, "#game-reader [data-verse='1'] .save-verse[aria-pressed=true]")
    view |> element("#game-reader [data-verse='1'] .save-verse") |> render_click()
    assert SavedVerses.page(user.id).entries == []
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
             SavedVerses.page(user.id).entries

    assert has_element?(view, "#atlas-reader [data-verse='3'] .save-verse[aria-pressed=true]")
  end

  test "anonymous saves and revoked sessions cannot write" do
    {:ok, game} = Games.create("owner")
    Games.submit(game.id, "owner", 0, "1 Nephi", "1")
    {:ok, view, _} = live(browser(), "/game/#{game.id}")
    assert has_element?(view, "#answer-reveal a.save-verse[aria-label='Sign in to save verse']")
    render_click(view, "save-verse", %{"source" => "round", "index" => "0"})
    {user, token} = account("reader")
    {:ok, view, _} = live(browser(token), "/game/#{game.id}")
    Scripguessr.Accounts.revoke_session(token)
    render_click(view, "save-verse", %{"source" => "round", "index" => "0"})
    assert_redirect(view, "/account/sign-in")
    assert SavedVerses.page(user.id).entries == []
  end

  test "saved pages are bounded while bookmark keys include entries on later pages" do
    {user, token} = account("reader")

    for number <- 1..21 do
      SavedVerses.save(
        user.id,
        %{"book" => "1 Nephi", "chapter" => 1, "verse" => number, "text" => "Passage #{number}."},
        "fixture-v1"
      )
    end

    page = SavedVerses.page(user.id)
    assert length(page.entries) == 20
    assert page.more?
    refute Enum.any?(page.entries, &(&1.verse == 1))
    assert [%{verse: 1} = oldest] = SavedVerses.page(user.id, 2).entries
    assert MapSet.size(SavedVerses.keys(user)) == 21
    assert MapSet.member?(SavedVerses.keys(user), {"1 Nephi", 1, 1})

    {:ok, view, _} = live(browser(token), "/saved-verses")

    assert length(
             view
             |> element("#saved-verses")
             |> render()
             |> LazyHTML.from_fragment()
             |> LazyHTML.query("article")
             |> Enum.to_list()
           ) == 20

    view |> element("a", "Older") |> render_click()
    assert_patch(view, "/saved-verses?page=2")
    assert has_element?(view, "#saved-#{oldest.id}", "Passage 1.")
    refute has_element?(view, "a", "Older")
    view |> element("#saved-#{oldest.id} button") |> render_click()
    assert_patch(view, "/saved-verses?page=1")
    refute has_element?(view, "#saved-#{oldest.id}")
    refute has_element?(view, "a", "Older")
    assert length(SavedVerses.page(user.id).entries) == 20

    for value <- ["0", "-1", "invalid", "100001"] do
      {:ok, invalid_view, _} = live(browser(token), "/saved-verses?page=#{value}")
      refute has_element?(invalid_view, "a", "Newer")
      assert has_element?(invalid_view, ".saved-card", "Passage 21.")
    end
  end

  test "explicit save intent is idempotent and invalid saves return changesets" do
    {user, _} = account("reader")
    verse = %{"book" => "1 Nephi", "chapter" => 1, "verse" => 1, "text" => "Passage."}
    assert {:ok, true} = SavedVerses.set_saved(user.id, verse, "fixture-v1", true)
    assert {:ok, true} = SavedVerses.set_saved(user.id, verse, "fixture-v1", true)
    assert [_] = SavedVerses.page(user.id).entries
    assert {:ok, false} = SavedVerses.set_saved(user.id, verse, "fixture-v1", false)
    assert {:ok, false} = SavedVerses.set_saved(user.id, verse, "fixture-v1", false)

    assert {:error, %Ecto.Changeset{}} =
             SavedVerses.set_saved(user.id, %{verse | "text" => ""}, "fixture-v1", true)

    assert SavedVerses.page(user.id).entries == []
  end

  test "a failed save displays an error without marking the verse saved" do
    {user, token} = account("reader")
    {:ok, game} = Games.create("owner", user.id)
    {:ok, game} = Games.submit(game.id, "owner", 0, "1 Nephi", "1", user.id)

    snapshot =
      update_in(
        game.snapshot,
        ["rounds"],
        &List.update_at(&1, 0, fn round -> Map.put(round, "text", "") end)
      )

    game |> Ecto.Changeset.change(snapshot: snapshot) |> Scripguessr.Repo.update!()
    {:ok, view, _} = live(browser(token), "/game/#{game.id}")
    view |> element("#answer-reveal .save-verse") |> render_click()
    assert has_element?(view, "#answer-reveal [role=alert]", "Couldn’t update")
    assert has_element?(view, "#answer-reveal .save-verse[aria-pressed=false]")
    assert SavedVerses.page(user.id).entries == []
  end
end
