defmodule ScripguessrWeb.DailyGameLiveTest do
  use Scripguessr.DataCase
  import Phoenix.ConnTest
  import Phoenix.LiveViewTest
  import Scripguessr.AccountFixtures
  import Scripguessr.GameFixtures
  alias Scripguessr.{Games, Repo}
  @endpoint ScripguessrWeb.Endpoint

  defp browser(token \\ nil),
    do:
      build_conn() |> Plug.Test.init_test_session(%{"owner" => "owner", "account_token" => token})

  test "daily entry is public, requires sign-in, and does not expose any prompts" do
    {:ok, home, _} = live(browser(), "/")
    assert has_element?(home, "#daily-game[href='/daily']")
    {:ok, daily, _} = live(browser(), "/daily")
    assert has_element?(daily, "#daily-sign-in[href='/account/sign-in']")
    assert has_element?(daily, ".site-nav a[href='/'][aria-current=page]")
    refute has_element?(daily, "#start-daily")
    refute has_element?(daily, "#verse-text")
    render_click(daily, "start-daily")
    assert_redirect(daily, "/account/sign-in")
  end

  test "start, resume, complete, share, and review the same ranked attempt" do
    {user, token} = account("reader")
    {:ok, daily, _} = live(browser(token), "/daily")
    daily |> element("#start-daily") |> render_click()
    {path, _} = assert_redirect(daily, 3_000)
    assert path =~ "/game/"
    game = Games.daily_for_user(Date.utc_today(), user.id)
    assert path == "/game/#{game.id}"
    {:ok, lobby, _} = live(browser(token), "/daily")
    assert has_element?(lobby, "#daily-attempt[href='#{path}']", "Continue daily")
    refute has_element?(lobby, "#start-daily")

    {:ok, view, html} = live(browser(token), path)
    assert has_element?(view, "#daily-game-label", "Daily game")
    first = hd(game.snapshot["rounds"])
    refute html =~ "#{first["book"]} #{first["chapter"]}:#{first["verse"]}"
    render_click(view, "read-chapter", %{"round" => "4"})
    refute has_element?(view, "#game-reader-title")
    render_click(view, "save-verse", %{"source" => "round", "index" => "4"})
    assert Scripguessr.SavedVerses.page(user.id).entries == []

    for round <- game.snapshot["rounds"] do
      view |> element("#book-grid button", round["book"]) |> render_click()

      unless has_element?(view, "#guess-form") do
        view
        |> element("#chapter-grid button[phx-value-chapter='#{round["chapter"]}']")
        |> render_click()
      end

      view |> form("#guess-form") |> render_submit()
      view |> element("#next-round") |> render_click()
    end

    assert has_element?(view, "#daily-results", "Rank #1")
    assert has_element?(view, "#daily-result", "ScripGuessr Daily")
    assert has_element?(view, "#daily-result", "🟩🟩🟩🟩🟩")
    refute view |> element("#daily-result") |> render() =~ first["text"]
    refute view |> element("#daily-result") |> render() =~ game.id
    assert has_element?(view, "#copy-daily-result[phx-hook=CopyResult]")
    assert has_element?(view, "#game-xp-reward", "+200 XP")
    {:ok, lobby, _} = live(browser(token), "/daily")
    assert has_element?(lobby, "#daily-attempt[href='#{path}']", "Review results")
    {:ok, board, _} = live(browser(token), "/leaderboard?board=daily")
    assert has_element?(board, "#leaderboard .leaderboard-self", "reader")
    assert has_element?(board, "#leaderboard .leaderboard-xp", "5,000")

    assert has_element?(
             board,
             "[aria-label='Leaderboard category'] a[aria-current=page]",
             "Daily game"
           )

    refute has_element?(board, "a[href='#{path}']")
    assert has_element?(view, "#play-again", "Random game")
    view |> element("#play-again") |> render_click()
    {random_path, _} = assert_redirect(view, 3_000)

    {:ok, random} =
      Games.fetch(String.replace_prefix(random_path, "/game/", ""), "owner", user.id)

    refute random.challenge_id
    assert Games.daily_for_user(Date.utc_today(), user.id).id == game.id
  end

  test "failed preparation offers retry without consuming the attempt" do
    {_, token} = account("reader")
    Scripguessr.Corpus.Fake.reset(:down)
    {:ok, view, _} = live(browser(token), "/daily")
    view |> element("#start-daily") |> render_click()
    render_async(view)
    assert has_element?(view, "[role=alert]", "couldn’t load")
    assert has_element?(view, "#start-daily:not([disabled])")
    Scripguessr.Corpus.Fake.reset()
    view |> element("#start-daily") |> render_click()
    assert_redirect(view, 3_000)
  end

  test "expired attempts are read-only and completed past results stay reviewable" do
    {user, token} = account("reader")
    {:ok, game} = Games.start_daily("owner", user.id)
    Repo.update!(Ecto.Changeset.change(game.challenge, day: Date.add(Date.utc_today(), -1)))
    {:ok, expired, _} = live(browser(token), "/game/#{game.id}")
    assert has_element?(expired, "#daily-expired")
    refute has_element?(expired, "#book-grid")
    refute has_element?(expired, "#verse-text")
    {:ok, fresh} = Games.start_daily("owner", user.id)
    finish_game(fresh, "owner", user.id)
    Repo.update!(Ecto.Changeset.change(fresh.challenge, day: Date.add(Date.utc_today(), -2)))
    {:ok, results, _} = live(browser(token), "/game/#{fresh.id}")
    assert has_element?(results, "#daily-results", "Rank #1")
    assert has_element?(results, ".final-score", "5000")
  end

  test "date selection uses URL state and malformed or future dates fall back to today" do
    {:ok, view, _} = live(browser(), "/leaderboard?board=daily&date=invalid")
    assert has_element?(view, "#daily-date[value='#{Date.utc_today()}']")
    yesterday = Date.add(Date.utc_today(), -1) |> Date.to_iso8601()
    view |> form("#daily-board-date", %{date: yesterday}) |> render_submit()
    path = assert_patch(view)
    assert URI.parse(path).path == "/leaderboard"
    assert URI.decode_query(URI.parse(path).query) == %{"board" => "daily", "date" => yesterday}
    assert has_element?(view, "#daily-date[value='#{yesterday}']")
    assert has_element?(view, "#leaderboard-empty", "No completed daily games")
    {:ok, future, _} = live(browser(), "/leaderboard?board=daily&date=9999-12-31")
    assert has_element?(future, "#daily-date[value='#{Date.utc_today()}']")
    {:ok, malformed, _} = live(browser(), "/leaderboard?board=daily&date[]=bad")
    assert has_element?(malformed, "#daily-date[value='#{Date.utc_today()}']")
  end
end
