defmodule ScripguessrWeb.AccountTest do
  use Scripguessr.DataCase
  import Phoenix.ConnTest
  import Phoenix.LiveViewTest
  import Scripguessr.AccountFixtures
  alias Scripguessr.{Accounts, Games, Repo}
  @endpoint ScripguessrWeb.Endpoint

  defp browser(owner, session \\ nil) do
    build_conn() |> Plug.Test.init_test_session(%{"owner" => owner, "account_token" => session})
  end

  test "registration shows a recovery code once and establishes a private session" do
    conn = browser("guest")
    assert get(conn, "/account/sign-in") |> html_response(200) =~ "Sign in"
    assert get(conn, "/account/register") |> html_response(200) =~ "Create an account"
    response = post(conn, "/account/register", credentials("Reader"))
    html = html_response(response, 200)
    assert html =~ "Save your recovery code"
    [_, code] = Regex.run(~r/id="recovery-code"[^>]*>([^<]+)</, html)
    assert byte_size(String.trim(code)) == 43
    session = Plug.Conn.get_session(response, :account_token)
    assert Accounts.user_for_session(session).username == "Reader"
    assert Plug.Conn.get_session(response, :owner) == "guest"
    assert Plug.Conn.get_session(response, :live_socket_id) == Accounts.session_topic(session)
    assert Plug.Conn.get_resp_header(response, "cache-control") == ["no-store"]
    assert Plug.Conn.get_resp_header(response, "referrer-policy") == ["no-referrer"]
    html = get(browser("device", session), "/account") |> html_response(200)
    assert html =~ "Reader"
    refute html =~ String.trim(code)
    signed_out = post(browser("device", session), "/account/sign-out")
    assert redirected_to(signed_out) == "/"
    assert is_nil(Accounts.user_for_session(session))
    assert is_nil(Plug.Conn.get_session(signed_out, :account_token))
    sign_in = post(conn, "/account/sign-in", %{username: "reader", password: password()})
    assert redirected_to(sign_in) == "/account"
    assert Accounts.user_for_session(Plug.Conn.get_session(sign_in, :account_token))
  end

  test "recovery resets the password, rotates the code and signs out other sessions" do
    {:ok, {user, old_session, code}} = Accounts.register(credentials("Reader"))

    assert get(browser("guest"), "/account/recover") |> html_response(200) =~
             "Recover your account"

    response =
      post(
        browser("guest"),
        "/account/recover",
        credentials("reader", "a completely different password") |> Map.put(:recovery_code, code)
      )

    html = html_response(response, 200)
    assert html =~ "Save your recovery code"
    refute html =~ code
    new_session = Plug.Conn.get_session(response, :account_token)
    assert Accounts.user_for_session(new_session).id == user.id
    assert is_nil(Accounts.user_for_session(old_session))

    rejected =
      post(
        browser("guest"),
        "/account/recover",
        credentials("reader") |> Map.put(:recovery_code, code)
      )

    assert html_response(rejected, 422) =~ "Invalid username or recovery code"

    assert post(browser("guest"), "/account/sign-in", %{username: "reader", password: password()})
           |> html_response(422) =~ "Invalid username or password"
  end

  test "account pages require a session and malformed credentials fail cleanly" do
    assert get(browser("guest"), "/account") |> redirected_to() == "/account/sign-in"

    assert post(browser("guest"), "/account/claim-games") |> redirected_to() ==
             "/account/sign-in"

    for params <- [
          %{},
          %{username: %{unexpected: "map"}, password: password()},
          %{username: "reader", password: %{unexpected: "map"}}
        ] do
      assert post(browser("guest"), "/account/sign-in", params) |> html_response(422) =~
               "Invalid username or password"

      assert post(browser("guest"), "/account/register", params) |> html_response(422) =~
               "account"
    end

    for _ <- 1..10, do: Accounts.RateLimit.allow?({127, 0, 0, 1})

    assert post(browser("guest"), "/account/sign-in", %{username: "reader", password: password()})
           |> html_response(429) =~ "Too many attempts"
  end

  test "attaching anonymous games is explicit, limited to this browser, and cannot be repeated by another account" do
    {user, session} = account("reader")
    {other, other_session} = account("other")
    {:ok, guest_game} = Games.create("guest")
    {:ok, stranger_game} = Games.create("stranger")
    before = get(browser("guest", session), "/account") |> html_response(200)
    assert before =~ "Keep this browser"
    assert before =~ "claim-games-form"
    refute before =~ "/game/#{guest_game.id}"
    assert {:ok, _} = Games.fetch(guest_game.id, "guest")

    response =
      post(browser("guest", session), "/account/claim-games", %{
        user_id: other.id,
        owner: "stranger"
      })

    assert redirected_to(response) == "/account"
    assert {:ok, claimed} = Games.fetch(guest_game.id, "new-device", user.id)
    assert claimed.snapshot == guest_game.snapshot
    assert {:error, :not_found} = Games.fetch(guest_game.id, "guest")
    assert {:error, :not_found} = Games.fetch(guest_game.id, "guest", other.id)
    assert {:ok, _} = Games.fetch(stranger_game.id, "stranger")
    post(browser("guest", other_session), "/account/claim-games")
    assert {:ok, _} = Games.fetch(guest_game.id, "new-device", user.id)
    assert Games.unclaimed_count("guest") == 0

    assert get(browser("new-device", session), "/account") |> html_response(200) =~
             "/game/#{guest_game.id}"

    refute get(browser("new-device", other_session), "/account") |> html_response(200) =~
             "/game/#{guest_game.id}"
  end

  test "signed-in games resume across devices and stay private after sign-out" do
    {user, session} = account("reader")
    {:ok, view, _} = live(browser("first-device", session), "/")
    view |> element("#start-game") |> render_click()
    {path, _} = assert_redirect(view, 1_000)
    id = String.replace_prefix(path, "/game/", "")
    assert {:ok, game} = Games.fetch(id, "second-device", user.id)
    assert game.user_id == user.id
    {:ok, second, _} = live(browser("second-device", session), path)
    second |> element("#book-grid button", "1 Nephi") |> render_click()
    second |> element("#chapter-grid button[phx-value-chapter='1']") |> render_click()
    second |> form("#guess-form") |> render_submit()
    {:ok, first, _} = live(browser("first-device", session), path)
    assert has_element?(first, "#answer-reveal")
    assert {:error, :not_found} = Games.fetch(id, "first-device")
    {:ok, guest, _} = live(browser("first-device"), path)
    assert has_element?(guest, "h1", "Game unavailable")
    Accounts.revoke_session(session)
    # A revoked or expired account cannot submit a new event through an old socket.
    render_click(first, "advance", %{"round" => "0"})
    assert_redirect(first, "/account/sign-in")
    saved = Repo.get!(Scripguessr.Games.Game, id)
    assert saved.snapshot["current"] == 0
  end

  test "history is private, paginated, and distinguishes completed from resumable games" do
    {user, session} = account("reader")
    {other, _} = account("other")
    {:ok, game} = Games.create("guest", user.id)
    complete = Map.put(game.snapshot, "finished", true)
    now = DateTime.utc_now()

    for i <- 1..21 do
      Repo.insert!(%Scripguessr.Games.Game{
        owner_hash: :crypto.hash(:sha256, "guest"),
        user_id: user.id,
        snapshot: if(i == 21, do: complete, else: game.snapshot),
        inserted_at: DateTime.add(now, i)
      })
    end

    {:ok, private} = Games.create("guest", other.id)
    response = get(browser("guest", session), "/account") |> html_response(200)
    assert response =~ "Completed game"
    assert response =~ "Game in progress"
    assert response =~ "Resume"
    assert response =~ "Review"
    assert response =~ "Older"
    refute response =~ "/game/#{private.id}"
    older = get(browser("guest", session), "/account?page=2") |> html_response(200)
    assert older =~ "/game/#{game.id}"
    assert older =~ "Newer"
    refute older =~ "Older"

    assert get(browser("guest", session), "/account?page=invalid") |> html_response(200) =~
             "Older"

    assert get(browser("guest", session), "/account?page[x]=invalid") |> html_response(200) =~
             "Older"
  end

  test "account mutations reject requests without a CSRF token" do
    conn = browser("guest") |> Plug.Conn.put_private(:plug_skip_csrf_protection, false)

    assert_raise Plug.CSRFProtection.InvalidCSRFTokenError, fn ->
      post(conn, "/account/sign-in", %{username: "reader", password: password()})
    end
  end

  test "forwarded client addresses are trusted only behind the configured loopback proxy" do
    on_exit(fn -> Application.delete_env(:scripguessr, :trust_loopback_proxy) end)
    conn = browser("guest") |> Plug.Conn.put_req_header("x-real-ip", "198.51.100.42")
    assert get(conn, "/account/sign-in").remote_ip == {127, 0, 0, 1}
    Application.put_env(:scripguessr, :trust_loopback_proxy, true)
    assert get(conn, "/account/sign-in").remote_ip == {198, 51, 100, 42}
    external = %{conn | remote_ip: {203, 0, 113, 42}}
    assert get(external, "/account/sign-in").remote_ip == {203, 0, 113, 42}
  end
end
