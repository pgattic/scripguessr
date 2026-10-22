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
    assert render_async(view) =~ "couldn’t prepare"
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

    for {round, index} <- Enum.with_index(quiz.snapshot["rounds"]) do
      assert has_element?(view, "#verse-text", round["text"])
      view |> form("#guess-form", guess: %{book: round["book"]}) |> render_change()

      view
      |> form("#guess-form",
        guess: %{
          book: round["book"],
          chapter: to_string(round["chapter"]),
          round: to_string(index)
        }
      )
      |> render_submit()

      assert has_element?(view, "#answer-reveal", "RIGHT AT HOME")
      view |> element("#next-round") |> render_click()
    end

    assert render(view) =~ "A little better acquainted."
    assert render(view) =~ "5000"
    {:ok, _view, html} = live(conn("owner"), "/quiz/#{quiz.id}")
    assert html =~ "A little better acquainted."
  end

  test "a copied URL shows no prompt to a different browser" do
    {:ok, quiz} = Scripguessr.Quizzes.create("owner")
    {:ok, _view, html} = live(conn("stranger"), "/quiz/#{quiz.id}")
    assert html =~ "This quiz isn’t available."
    refute html =~ hd(quiz.snapshot["rounds"])["text"]
  end

  test "malformed round events leave the saved quiz playable" do
    {:ok, quiz} = Scripguessr.Quizzes.create("owner")
    {:ok, view, _} = live(conn("owner"), "/quiz/#{quiz.id}")

    render_click(view, "guess", %{
      "guess" => %{"book" => "1 Nephi", "chapter" => "1", "round" => nil}
    })

    render_click(view, "advance", %{"round" => -1})
    assert has_element?(view, "#guess-form")
    assert {:ok, saved} = Scripguessr.Quizzes.fetch(quiz.id, "owner")
    assert saved.snapshot == quiz.snapshot
  end

  test "health endpoints distinguish readiness" do
    assert build_conn() |> get("/healthz") |> response(200) == "ok"
    assert build_conn() |> get("/readyz") |> response(200) == "ready"
  end
end
