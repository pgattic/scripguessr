defmodule ScripguessrWeb.MatchLiveTest do
  use Scripguessr.DataCase
  import Phoenix.ConnTest
  import Phoenix.LiveViewTest
  import Scripguessr.AccountFixtures
  import Scripguessr.MatchFixtures
  alias Scripguessr.{Matches, Accounts, Repo}
  alias Scripguessr.Matches.Match
  @endpoint ScripguessrWeb.Endpoint

  defp browser(token \\ nil),
    do:
      build_conn() |> Plug.Test.init_test_session(%{"owner" => "owner", "account_token" => token})

  test "Play links to matches and guests return to their invite after signing in" do
    {code, [{_host, _}, {friend, _}]} = match_lobby()
    {:ok, home, _} = live(browser(), "/")
    assert has_element?(home, "#online-matches[href='/matches']")
    {:ok, guest, html} = live(browser(), "/matches/#{code}")
    assert has_element?(guest, "#match-sign-in")
    refute html =~ "verse-text"

    sign_in =
      guest
      |> element("#match-sign-in")
      |> render()
      |> then(fn html ->
        [_, href] = Regex.run(~r/href="([^"]+)"/, html)
        href |> String.replace("&amp;", "&")
      end)

    conn = get(browser(), sign_in)

    conn =
      post(recycle(conn), "/account/sign-in", %{username: friend.username, password: password()})

    assert redirected_to(conn) == "/matches/#{code}"
    {:ok, invitation, _} = live(recycle(conn), "/matches/#{code}")
    assert has_element?(invitation, "#join-match")
  end

  test "room creation handles upstream failure and retry" do
    {_, token} = account("host")
    {:ok, view, _} = live(browser(token), "/matches")
    Scripguessr.Corpus.Fake.reset(:down)
    view |> element("#create-match") |> render_click()
    render_async(view)
    assert has_element?(view, "[role=alert]", "couldn’t load")
    Scripguessr.Corpus.Fake.reset()
    view |> element("#create-match") |> render_click()
    {path, _} = assert_redirect(view, 3_000)
    assert path =~ "/matches/"
    {:ok, room, _} = live(browser(token), path)
    assert has_element?(room, "#match-lobby")
    assert has_element?(room, "#start-match[disabled]")
    assert has_element?(room, "#copy-match-invite[phx-hook=CopyText]")
  end

  test "two clients join, ready, start, and see answers only after both submit" do
    {code, [{host, host_token}, {friend, friend_token}]} = match_lobby()
    {:ok, host_view, _} = live(browser(host_token), "/matches/#{code}")
    {:ok, friend_view, _} = live(browser(friend_token), "/matches/#{code}")
    friend_view |> element("#join-match") |> render_click()
    assert has_element?(host_view, "#match-players", friend.username)
    host_view |> element("#match-ready") |> render_click()
    friend_view |> element("#match-ready") |> render_click()
    assert has_element?(host_view, "#start-match:not([disabled])")
    host_view |> element("#start-match") |> render_click()
    assert has_element?(friend_view, "#verse-text")
    assert has_element?(host_view, "#match-countdown")
    round = hd(Repo.get_by!(Match, code: code).snapshot["rounds"])
    answer = "#{round["book"]} #{round["chapter"]}:#{round["verse"]}"
    refute render(host_view) =~ answer
    host_view |> element("#book-grid button", "Omni") |> render_click()
    assert has_element?(host_view, "#guess-form .ready", "Omni 1")
    refute has_element?(host_view, "#chapter-grid")
    host_view |> form("#guess-form") |> render_submit()
    assert has_element?(host_view, "#picker", "Guess locked")
    refute has_element?(host_view, "#match-round-result")
    refute render(host_view) =~ answer
    assert has_element?(friend_view, "#book-grid")
    render_click(host_view, "read-chapter", %{"round" => "4"})
    render_click(host_view, "save-verse", %{"source" => "round", "index" => "4"})
    refute has_element?(host_view, "#match-reader")
    assert Scripguessr.SavedVerses.page(host.id).entries == []
    friend_view |> element("#book-grid button", round["book"]) |> render_click()

    unless has_element?(friend_view, "#guess-form") do
      friend_view
      |> element("#chapter-grid button[phx-value-chapter='#{round["chapter"]}']")
      |> render_click()
    end

    friend_view |> form("#guess-form") |> render_submit()
    assert has_element?(host_view, "#match-round-result", answer)
    assert has_element?(friend_view, "#match-round-result", answer)
    refute has_element?(friend_view, "#book-grid")
    refute has_element?(host_view, "#match-countdown")
    assert has_element?(host_view, "#advance-match", "Next round")
    assert has_element?(friend_view, "#waiting-for-host")
    refute has_element?(friend_view, "#advance-match")
    render_click(friend_view, "advance", %{"round" => "0"})
    assert has_element?(friend_view, "#match-round-result", answer)
    host_view |> element("#advance-match") |> render_click()
    assert has_element?(host_view, "#book-grid")
    assert has_element?(friend_view, "#book-grid")
    assert has_element?(host_view, ".match-heading", "Round 2 of 5")
    assert has_element?(friend_view, ".match-heading", "Round 2 of 5")
    assert Matches.fetch(code, friend.id) |> elem(1) |> Map.fetch!(:round) == 1
  end

  test "reconnect restores a locked guess without revealing its score or answer" do
    {code, [{host, token}, _], snapshot} = started_match()
    first = hd(snapshot["rounds"])
    Matches.submit(code, host.id, 0, first["book"], to_string(first["chapter"]))
    {:ok, view, html} = live(browser(token), "/matches/#{code}")
    assert has_element?(view, "#match-waiting", "#{first["book"]} #{first["chapter"]}")
    refute html =~ "#{first["book"]} #{first["chapter"]}:#{first["verse"]}"
    refute has_element?(view, "#match-round-result")
    refute has_element?(view, "#guess-form")
    expire_match(code)
    assert has_element?(view, "#match-round-result")
  end

  test "completed match results persist, ties are explicit, and outsiders cannot see results" do
    {code, [{host, token}, {_friend, friend_token}], _} = started_match()
    match = Repo.get_by!(Match, code: code)

    for index <- 0..3 do
      expire_match(code)
      {:ok, _} = Matches.advance(code, host.id, index)
    end

    expire_match(code)
    {:ok, results, _} = live(browser(token), "/matches/#{code}")
    {:ok, friend_view, _} = live(browser(friend_token), "/matches/#{code}")
    assert has_element?(results, ".match-heading", "Round 5 results")
    assert has_element?(results, "#match-round-result")
    assert has_element?(results, "#advance-match", "View results")
    refute has_element?(results, "#match-results")
    assert has_element?(friend_view, "#waiting-for-host", "show the results")
    results |> element("#advance-match") |> render_click()
    assert has_element?(friend_view, "#match-results")
    assert has_element?(results, ".match-heading", "It’s a tie!")
    assert has_element?(results, "#match-results")

    assert has_element?(results, "#match-results .match-result-round:nth-child(5)")
    refute has_element?(results, "#match-results .match-result-round:nth-child(6)")

    assert has_element?(results, "#match-results button[phx-click=read-chapter]")
    refute has_element?(results, "#guess-form")
    results |> element("#match-results button[phx-value-round='0']") |> render_click()
    render_async(results)
    assert has_element?(results, "#match-reader")
    results |> element("#match-reader-close") |> render_click()

    {:ok, outsider, html} = live(browser(), "/matches/#{code}")
    refute has_element?(outsider, "#match-results")
    render_click(outsider, "read-chapter", %{"round" => "0"})
    refute has_element?(outsider, "#match-reader")

    refute html =~ hd(match.snapshot["rounds"])["text"]
    {:ok, hub, _} = live(browser(token), "/matches")
    assert has_element?(hub, "#recent-matches a[href='/matches/#{code}']", "View results")
    {:ok, refreshed, _} = live(browser(token), "/matches/#{code}")
    assert has_element?(refreshed, "#match-results")
    assert Scripguessr.Progression.for_user(host.id).xp == 0
  end

  test "revealed chapters support reading and saving, and advancing closes every reader" do
    {code, [{host, host_token}, {_friend, friend_token}], snapshot} = started_match()
    first = hd(snapshot["rounds"])
    {:ok, host_view, _} = live(browser(host_token), "/matches/#{code}")
    {:ok, friend_view, _} = live(browser(friend_token), "/matches/#{code}")

    for index <- ["0", "4"] do
      render_click(host_view, "read-chapter", %{"round" => index})
      render_click(host_view, "save-verse", %{"source" => "round", "index" => index})
      refute has_element?(host_view, "#match-reader")
    end

    assert Scripguessr.SavedVerses.page(host.id).entries == []
    expire_match(code)
    assert has_element?(host_view, "#answer-reveal", "No answer submitted")
    host_view |> element("#read-chapter") |> render_click()
    render_async(host_view)
    assert has_element?(host_view, "#match-reader-title", "#{first["book"]} #{first["chapter"]}")
    assert has_element?(host_view, "#match-reader .matched[data-verse='#{first["verse"]}']")
    number = if first["verse"] == 3, do: 1, else: 3

    host_view
    |> element("#match-reader .save-verse[phx-value-verse='#{number}']")
    |> render_click()

    [saved] = Scripguessr.SavedVerses.page(host.id).entries
    assert saved.book == first["book"]
    assert saved.chapter == first["chapter"]
    assert saved.verse == number
    assert saved.text == "Invented study passage #{number}."
    host_view |> element("#match-reader-close") |> render_click()
    host_view |> element("#answer-reveal .save-verse") |> render_click()
    assert has_element?(host_view, "#answer-reveal .save-verse[aria-pressed=true]")

    friend_view |> element("#read-chapter") |> render_click()
    render_async(friend_view)
    assert has_element?(friend_view, "#match-reader")
    host_view |> element("#advance-match") |> render_click()
    refute has_element?(friend_view, "#match-reader")
    assert has_element?(friend_view, "#book-grid")
    render_click(friend_view, "read-chapter", %{"round" => "0"})
    render_click(friend_view, "save-verse", %{"source" => "reader", "verse" => "3"})
    refute has_element?(friend_view, "#match-reader")
  end

  test "an in-flight chapter cannot appear after the host advances" do
    {code, [{host, token}, _], snapshot} = started_match()
    expire_match(code)
    {:ok, view, _} = live(browser(token), "/matches/#{code}")
    Scripguessr.Corpus.Fake.block_next("atlas:reader", self())
    view |> element("#read-chapter") |> render_click()
    assert_receive {:corpus_blocked, _, worker}
    on_exit(fn -> Scripguessr.Corpus.Chapter.clear() end)
    assert has_element?(view, "#match-reader")
    {:ok, _} = Matches.advance(code, host.id, 0)
    refute has_element?(view, "#match-reader")
    send(worker, :release_corpus)
    first = hd(snapshot["rounds"])

    assert {:ok, _} =
             Scripguessr.Corpus.Chapter.read(first["book"], first["chapter"], snapshot["release"])

    render_async(view)
    refute has_element?(view, "#match-reader")
    assert has_element?(view, "#book-grid")
  end

  test "chapter failures can be retried without affecting match scores" do
    {code, [{_host, token}, _], _} = started_match()
    expire_match(code)
    {:ok, view, _} = live(browser(token), "/matches/#{code}")
    Scripguessr.Corpus.Fake.reset(:down)
    view |> element("#read-chapter") |> render_click()
    render_async(view)
    assert has_element?(view, "#match-reader [role=alert]", "Couldn’t load")
    Scripguessr.Corpus.Fake.reset()
    render_click(view, "retry-chapter")
    render_async(view)
    assert has_element?(view, "#match-reader .atlas-reading-verse")
    assert has_element?(view, "#answer-reveal .score", "0")
  end

  test "session revocation prevents further pushed match content" do
    {code, [{_host, token}, _], _} = started_match()
    {:ok, view, _} = live(browser(token), "/matches/#{code}")
    Accounts.revoke_session(token)
    id = Repo.get_by!(Match, code: code).id
    send(view.pid, {:match_updated, id})
    assert_redirect(view, "/account/sign-in")
  end

  test "idle room lists and active matches do not poll the database per viewer" do
    {code, [{_host, token}, _], _} = started_match()
    {:ok, hub, _} = live(browser(token), "/matches")
    {:ok, room, _} = live(browser(token), "/matches/#{code}")
    # Flush initial presence updates before measuring steady-state activity.
    render(room)
    id = "match-idle-#{System.unique_integer([:positive])}"
    viewers = [hub.pid, room.pid]

    :ok =
      :telemetry.attach(
        id,
        [:scripguessr, :repo, :query],
        &__MODULE__.record_viewer_query/4,
        {self(), viewers}
      )

    on_exit(fn -> :telemetry.detach(id) end)
    refute_receive {:viewer_query, _}, 1_250
    assert has_element?(room, "#match-countdown[phx-hook=MatchCountdown][data-deadline]")
    render_click(room, "clear-guess")
    assert_receive {:viewer_query, _}
  end

  def record_viewer_query(_event, _measurements, metadata, {test_pid, viewers}) do
    if self() in viewers, do: send(test_pid, {:viewer_query, metadata.query})
  end

  test "registration preserves an invite destination without skipping the recovery code" do
    code = String.duplicate("a", 24)

    conn =
      get(browser(), "/account/register?" <> URI.encode_query(%{return_to: "/matches/#{code}"}))

    conn = post(recycle(conn), "/account/register", credentials("newplayer"))
    html = html_response(conn, 200)
    assert html =~ "Save your recovery code"
    assert html =~ "href=\"/matches/#{code}\""
  end

  test "sign-in return destinations reject external and malformed paths" do
    {user, _} = account("reader")

    for path <- [
          "https://example.org",
          "//example.org",
          "/matches/" <> String.duplicate("a", 24) <> "\n",
          "/account/sign-out"
        ] do
      conn = get(browser(), "/account/sign-in?" <> URI.encode_query(%{return_to: path}))

      conn =
        post(recycle(conn), "/account/sign-in", %{username: user.username, password: password()})

      assert redirected_to(conn) == "/account"
    end
  end
end
