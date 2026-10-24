defmodule ScripguessrWeb.GameLiveTest do
  use Scripguessr.DataCase
  import Phoenix.ConnTest
  import Phoenix.LiveViewTest
  @endpoint ScripguessrWeb.Endpoint
  defp conn(owner), do: build_conn() |> Plug.Test.init_test_session(%{"owner" => owner})

  test "home prepares a game asynchronously and offers retry on upstream failure" do
    {:ok, view, _} = live(conn("owner"), "/")
    assert has_element?(view, "#start-game")
    Scripguessr.Corpus.Fake.reset(:down)
    view |> element("#start-game") |> render_click()
    assert render_async(view) =~ "couldn’t load"
    Scripguessr.Corpus.Fake.reset()
    view |> element("#start-game") |> render_click()
    {path, _flash} = assert_redirect(view, 1_000)
    assert path =~ "/game/"
  end

  test "all five rounds reveal only submitted answers and finish in a summary" do
    {:ok, game} = Scripguessr.Games.create("owner")
    {:ok, view, html} = live(conn("owner"), "/game/#{game.id}")
    first = hd(game.snapshot["rounds"])
    refute html =~ "#{first["book"]} #{first["chapter"]}:#{first["verse"]}"

    for round <- game.snapshot["rounds"] do
      assert has_element?(view, "#verse-text", round["text"])
      view |> element("#book-grid button", round["book"]) |> render_click()

      view
      |> element("#chapter-grid button[phx-value-chapter='#{round["chapter"]}']")
      |> render_click()

      view |> form("#guess-form") |> render_submit()

      assert has_element?(view, "#answer-reveal", "Exact chapter")
      view |> element("#next-round") |> render_click()
    end

    assert render(view) =~ "Final score"
    assert render(view) =~ "5000"
    view |> element(".round-list button[phx-value-round='0']") |> render_click()
    render_async(view)
    assert has_element?(view, "#game-reader-title", "#{first["book"]} #{first["chapter"]}")
    assert has_element?(view, "#game-reader .matched[data-verse='#{first["verse"]}']")
    view |> element("#game-reader-close") |> render_click()
    assert render(view) =~ "Final score"
    {:ok, _view, html} = live(conn("owner"), "/game/#{game.id}")
    assert html =~ "Final score"
  end

  test "a copied URL shows no prompt to a different browser" do
    {:ok, game} = Scripguessr.Games.create("owner")
    {:ok, _view, html} = live(conn("stranger"), "/game/#{game.id}")
    assert html =~ "Game unavailable"
    refute html =~ hd(game.snapshot["rounds"])["text"]
  end

  test "breadcrumbs let the player revise a selection before submitting" do
    {:ok, game} = Scripguessr.Games.create("owner")
    {:ok, view, _} = live(conn("owner"), "/game/#{game.id}")
    assert has_element?(view, "[role=progressbar][aria-valuenow='0']")
    refute has_element?(view, ".progress-track .filled")
    view |> element("#book-grid button", "1 Nephi") |> render_click()
    view |> element("#chapter-grid button[phx-value-chapter='3']") |> render_click()
    assert has_element?(view, "#guess-form .ready", "1 Nephi 3")
    view |> element("#edit-book") |> render_click()
    assert has_element?(view, "#chapter-grid button.active[phx-value-chapter='3']")
    refute has_element?(view, "#book-grid")
    assert has_element?(view, "#edit-book.current")
    view |> element("#edit-chapter") |> render_click()
    assert has_element?(view, "#guess-form .ready", "1 Nephi 3")
    assert has_element?(view, "#edit-chapter.current")
    view |> element("#edit-book") |> render_click()
    view |> element("#chapter-grid button[phx-value-chapter='4']") |> render_click()
    assert has_element?(view, "#guess-form .ready", "1 Nephi 4")
    view |> element("#clear-guess") |> render_click()
    assert has_element?(view, "#book-grid")
    refute has_element?(view, "#edit-book")
    refute has_element?(view, "#edit-chapter")
    view |> element("#book-grid button", "2 Nephi") |> render_click()
    refute has_element?(view, "#guess-form")
    view |> element("#chapter-grid button[phx-value-chapter='1']") |> render_click()
    view |> form("#guess-form") |> render_submit()
    assert has_element?(view, "#answer-reveal", "2 Nephi 1")
    assert has_element?(view, "[role=progressbar][aria-valuenow='1']")
    assert has_element?(view, ".progress-track .filled:first-child")
    refute has_element?(view, ".progress-track .filled:nth-child(2)")
    render_click(view, "choose-book", %{"book" => "1 Nephi"})
    render_click(view, "clear-guess")
    assert has_element?(view, "#answer-reveal", "2 Nephi 1")
    view |> element("#next-round") |> render_click()
    assert has_element?(view, "#book-grid")
    assert has_element?(view, "[role=progressbar][aria-valuenow='1']")
    refute has_element?(view, ".progress-track .filled:nth-child(2)")
  end

  test "malformed round events leave the saved game playable" do
    {:ok, game} = Scripguessr.Games.create("owner")
    {:ok, view, _} = live(conn("owner"), "/game/#{game.id}")

    render_click(view, "guess", %{
      "guess" => %{"book" => "1 Nephi", "chapter" => "1", "round" => nil}
    })

    render_click(view, "advance", %{"round" => -1})
    assert has_element?(view, "#book-grid")
    assert {:ok, saved} = Scripguessr.Games.fetch(game.id, "owner")
    assert saved.snapshot == game.snapshot
  end

  test "health endpoints distinguish readiness" do
    assert build_conn() |> get("/healthz") |> response(200) == "ok"
    assert build_conn() |> get("/readyz") |> response(200) == "ready"
  end

  test "chapter reading is restricted to submitted rounds and uses the actual answer" do
    {:ok, game} = Scripguessr.Games.create("owner")
    {:ok, view, _} = live(conn("owner"), "/game/#{game.id}")
    first = hd(game.snapshot["rounds"])
    calls = Scripguessr.Corpus.Fake.calls()
    refute has_element?(view, "#read-chapter")

    for index <- ["0", "1", "-1", "100", "invalid", nil] do
      render_click(view, "read-chapter", %{"round" => index})
      refute has_element?(view, "#game-reader")
    end

    assert Scripguessr.Corpus.Fake.calls() == calls
    view |> element("#book-grid button", "2 Nephi") |> render_click()
    view |> element("#chapter-grid button[phx-value-chapter='1']") |> render_click()
    view |> form("#guess-form") |> render_submit()
    view |> element("#read-chapter") |> render_click()
    render_async(view)
    assert has_element?(view, "#game-reader-title", "#{first["book"]} #{first["chapter"]}")

    assert has_element?(
             view,
             "#game-reader .matched[data-verse='#{first["verse"]}'][aria-current='true']"
           )

    assert has_element?(view, "#game-reader [data-verse='3']", "Invented study passage 3.")
    assert has_element?(view, "[role=progressbar][aria-valuenow='1']")
    sql = List.last(Scripguessr.Corpus.Fake.calls())
    assert sql =~ "b.name='#{first["book"]}' and c.number=#{first["chapter"]}"
    view |> element("#game-reader-close") |> render_click()
    refute has_element?(view, "#game-reader")
    assert has_element?(view, "#answer-reveal", "2 Nephi 1")
    view |> element("#read-chapter") |> render_click()
    render_async(view)
    view |> element("#next-round") |> render_click()
    refute has_element?(view, "#game-reader")
    refute has_element?(view, "#read-chapter")
    assert has_element?(view, "#book-grid")
    assert has_element?(view, "[role=progressbar][aria-valuenow='1']")
  end

  test "chapter loading can fail and retry without changing the saved game" do
    {:ok, game} = Scripguessr.Games.create("owner")
    {:ok, game} = Scripguessr.Games.submit(game.id, "owner", 0, "1 Nephi", "1")
    {:ok, view, _} = live(conn("owner"), "/game/#{game.id}")
    Scripguessr.Corpus.Fake.reset(:down)
    view |> element("#read-chapter") |> render_click()
    render_async(view)
    assert has_element?(view, "#game-reader [role=alert]", "Couldn’t load this chapter")
    Scripguessr.Corpus.Fake.reset()
    view |> element("#game-reader button[phx-click='retry-chapter']") |> render_click()
    render_async(view)
    assert has_element?(view, "#game-reader .atlas-reading")
    assert {:ok, saved} = Scripguessr.Games.fetch(game.id, "owner")
    assert saved.snapshot == game.snapshot
    view |> element("#game-reader-close") |> render_click()
    Scripguessr.Corpus.Chapter.clear()
    Agent.update(Scripguessr.Corpus.Fake, &%{&1 | release: "new-release"})
    view |> element("#read-chapter") |> render_click()
    render_async(view)
    assert has_element?(view, "#game-reader [role=alert]", "corpus has changed")
    refute has_element?(view, "#game-reader .atlas-reading")
    view |> element("#game-reader-close") |> render_click()
    view |> element("#next-round") |> render_click()
    assert has_element?(view, "#book-grid")
  end
end
