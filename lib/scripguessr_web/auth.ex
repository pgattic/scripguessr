defmodule ScripguessrWeb.Auth do
  import Plug.Conn
  import Phoenix.Controller
  alias Scripguessr.{Accounts, Progression}

  def init(opts), do: opts

  def call(conn, _) do
    conn =
      if Application.get_env(:scripguessr, :trust_loopback_proxy, false) and
           conn.remote_ip in [{127, 0, 0, 1}, {0, 0, 0, 0, 0, 0, 0, 1}] do
        Plug.RewriteOn.call(conn, [{:remote_ip, "x-real-ip"}])
      else
        conn
      end

    user = Accounts.user_for_session(get_session(conn, :account_token))

    conn
    |> assign(:current_user, user)
    |> assign(:current_progress, progress(user))
    |> assign(:active_section, section(conn.request_path))
    |> put_resp_header("cache-control", "no-store")
    |> put_resp_header("referrer-policy", "no-referrer")
  end

  def require_user(conn, _) do
    if conn.assigns.current_user,
      do: conn,
      else: conn |> redirect(to: "/account/sign-in") |> halt()
  end

  def log_in(conn, user, token) do
    if old = get_session(conn, :account_token), do: Accounts.revoke_session(old)
    owner = get_session(conn, :owner)
    delete_csrf_token()

    conn
    |> configure_session(renew: true)
    |> clear_session()
    |> put_session(:owner, owner)
    |> put_session(:account_token, token)
    |> put_session(:live_socket_id, Accounts.session_topic(token))
    |> assign(:current_user, user)
    |> assign(:current_progress, progress(user))
  end

  def log_out(conn) do
    if token = get_session(conn, :account_token), do: Accounts.revoke_session(token)
    owner = get_session(conn, :owner)
    delete_csrf_token()
    conn |> configure_session(renew: true) |> clear_session() |> put_session(:owner, owner)
  end

  def on_mount(:current_user, _params, session, socket) do
    token = session["account_token"]
    user = Accounts.user_for_session(token)

    socket =
      Phoenix.Component.assign(socket,
        current_user: user,
        current_progress: progress(user),
        active_section: nil
      )

    socket =
      Phoenix.LiveView.attach_hook(socket, :navigation, :handle_params, fn _, uri, socket ->
        {:cont, Phoenix.Component.assign(socket, :active_section, section(URI.parse(uri).path))}
      end)

    # LiveViews can remain open past expiry or revocation. Recheck protected events,
    # including events submitted by a stale tab after another tab signs out.
    socket =
      if user do
        Phoenix.LiveView.attach_hook(socket, :account_session, :handle_event, fn _, _, socket ->
          if Accounts.user_for_session(token) do
            {:cont, socket}
          else
            {:halt, Phoenix.LiveView.redirect(socket, to: "/account/sign-in")}
          end
        end)
      else
        socket
      end

    {:cont, socket}
  end

  defp progress(nil), do: nil
  defp progress(user), do: Progression.for_user(user.id)
  defp section("/"), do: :play
  defp section("/daily"), do: :play
  defp section("/matches"), do: :play
  defp section("/matches/" <> _), do: :play
  defp section("/game/" <> _), do: :play
  defp section("/atlas"), do: :atlas
  defp section("/leaderboard"), do: :leaderboard
  defp section("/saved-verses"), do: :saved
  defp section(_), do: :account
end
