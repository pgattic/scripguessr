defmodule ScripguessrWeb.QuizLiveTest do
  use Scripguessr.DataCase
  import Phoenix.ConnTest
  import Phoenix.LiveViewTest
  @endpoint ScripguessrWeb.Endpoint
  defp conn(owner), do: build_conn() |> Plug.Test.init_test_session(%{"owner" => owner})

  test "home prepares a quiz asynchronously and offers retry on upstream failure" do
    {:ok, view, _} = live(conn("owner"), "/")
    assert has_element?(view, "#start-quiz")
    Scripguessr.Corpus.Fake.reset(:down)
    view |> element("#start-quiz") |> render_click()
    assert render_async(view) =~ "couldn’t load"
    Scripguessr.Corpus.Fake.reset()
    view |> element("#start-quiz") |> render_click()
    {path, _flash} = assert_redirect(view, 1_000)
    assert path =~ "/quiz/"
  end

  test "all five rounds reveal only submitted answers and finish in a summary" do
    {:ok, quiz} = Scripguessr.Quizzes.create("owner")
    {:ok, view, html} = live(conn("owner"), "/quiz/#{quiz.id}")
    first = hd(quiz.snapshot["rounds"])
    refute html =~ "#{first["book"]} #{first["chapter"]}:#{first["verse"]}"

    for round <- quiz.snapshot["rounds"] do
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
    {:ok, _view, html} = live(conn("owner"), "/quiz/#{quiz.id}")
    assert html =~ "Final score"
  end

  test "a copied URL shows no prompt to a different browser" do
    {:ok, quiz} = Scripguessr.Quizzes.create("owner")
    {:ok, _view, html} = live(conn("stranger"), "/quiz/#{quiz.id}")
    assert html =~ "Quiz unavailable"
    refute html =~ hd(quiz.snapshot["rounds"])["text"]
  end

  test "breadcrumbs let the player revise a selection before submitting" do
    {:ok, quiz} = Scripguessr.Quizzes.create("owner")
    {:ok, view, _} = live(conn("owner"), "/quiz/#{quiz.id}")
    assert has_element?(view, "[role=progressbar][aria-valuenow='0']")
    refute has_element?(view, ".progress-track .filled")
    view |> element("#book-grid button", "1 Nephi") |> render_click()
    view |> element("#chapter-grid button[phx-value-chapter='3']") |> render_click()
    assert has_element?(view, "#guess-form .ready", "1 Nephi 3")
    view |> element("#edit-chapter") |> render_click()
    assert has_element?(view, "#chapter-grid button.active[phx-value-chapter='3']")
    view |> element("#chapter-grid button[phx-value-chapter='4']") |> render_click()
    assert has_element?(view, "#guess-form .ready", "1 Nephi 4")
    view |> element("#edit-book") |> render_click()
    view |> element("#book-grid button", "2 Nephi") |> render_click()
    refute has_element?(view, "#guess-form")
    view |> element("#chapter-grid button[phx-value-chapter='1']") |> render_click()
    view |> form("#guess-form") |> render_submit()
    assert has_element?(view, "#answer-reveal", "2 Nephi 1")
    assert has_element?(view, "[role=progressbar][aria-valuenow='1']")
    assert has_element?(view, ".progress-track .filled:first-child")
    refute has_element?(view, ".progress-track .filled:nth-child(2)")
    render_click(view, "choose-book", %{"book" => "1 Nephi"})
    assert has_element?(view, "#answer-reveal", "2 Nephi 1")
    view |> element("#next-round") |> render_click()
    assert has_element?(view, "#book-grid")
    assert has_element?(view, "[role=progressbar][aria-valuenow='1']")
    refute has_element?(view, ".progress-track .filled:nth-child(2)")
  end

  test "malformed round events leave the saved quiz playable" do
    {:ok, quiz} = Scripguessr.Quizzes.create("owner")
    {:ok, view, _} = live(conn("owner"), "/quiz/#{quiz.id}")

    render_click(view, "guess", %{
      "guess" => %{"book" => "1 Nephi", "chapter" => "1", "round" => nil}
    })

    render_click(view, "advance", %{"round" => -1})
    assert has_element?(view, "#book-grid")
    assert {:ok, saved} = Scripguessr.Quizzes.fetch(quiz.id, "owner")
    assert saved.snapshot == quiz.snapshot
  end

  test "health endpoints distinguish readiness" do
    assert build_conn() |> get("/healthz") |> response(200) == "ok"
    assert build_conn() |> get("/readyz") |> response(200) == "ready"
  end
end
