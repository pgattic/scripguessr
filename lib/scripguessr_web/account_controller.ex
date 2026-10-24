defmodule ScripguessrWeb.AccountController do
  use Phoenix.Controller, formats: [:html]
  import Plug.Conn
  alias Scripguessr.{Accounts, Games}
  alias ScripguessrWeb.Auth

  plug :require_user when action in [:show, :claim]
  defp require_user(conn, opts), do: Auth.require_user(conn, opts)

  def new(conn, params), do: auth_page(remember_return(conn, params), :sign_in)
  def registration(conn, params), do: auth_page(remember_return(conn, params), :register)
  def recovery(conn, params), do: page(remember_return(conn, params), mode: :recover, error: nil)

  defp remember_return(conn, %{"return_to" => path}) do
    if is_binary(path) and
         (path in ["/matches", "/daily"] or Regex.match?(~r|\A/matches/[A-Za-z0-9_-]{24}\z|, path)),
       do: put_session(conn, :return_to, path),
       else: delete_session(conn, :return_to)
  end

  defp remember_return(conn, _), do: conn

  defp auth_page(conn, mode) do
    if conn.assigns.current_user do
      destination = get_session(conn, :return_to) || "/account"
      conn |> delete_session(:return_to) |> redirect(to: destination)
    else
      page(conn, mode: mode, error: nil)
    end
  end

  def register(conn, params) do
    limited(conn, :register, fn conn ->
      case Accounts.register(params) do
        {:ok, {user, token, code}} ->
          recovery_code_page(conn, user, token, code)

        {:error, %Ecto.Changeset{} = changeset} ->
          conn |> put_status(422) |> page(mode: :register, error: changeset_error(changeset))
      end
    end)
  end

  def sign_in(conn, params) do
    limited(conn, :sign_in, fn conn ->
      case Accounts.sign_in(params["username"], params["password"]) do
        {:ok, {user, token}} ->
          destination = get_session(conn, :return_to) || "/account"
          conn |> Auth.log_in(user, token) |> redirect(to: destination)

        {:error, _} ->
          conn |> put_status(422) |> page(mode: :sign_in, error: "Invalid username or password.")
      end
    end)
  end

  def recover(conn, params) do
    limited(conn, :recover, fn conn ->
      case Accounts.recover(params["username"], params["recovery_code"], params) do
        {:ok, {user, token, code}} ->
          recovery_code_page(conn, user, token, code)

        {:error, %Ecto.Changeset{} = changeset} ->
          conn |> put_status(422) |> page(mode: :recover, error: changeset_error(changeset))

        {:error, _} ->
          conn
          |> put_status(422)
          |> page(mode: :recover, error: "Invalid username or recovery code.")
      end
    end)
  end

  defp limited(conn, mode, action) do
    if Accounts.RateLimit.allow?(conn.remote_ip) do
      action.(conn)
    else
      conn
      |> put_status(429)
      |> put_resp_header("retry-after", "600")
      |> page(mode: mode, error: "Too many attempts. Try again in a few minutes.")
    end
  end

  defp recovery_code_page(conn, user, token, code) do
    destination = get_session(conn, :return_to) || "/account"

    conn
    |> Auth.log_in(user, token)
    |> page(mode: :recovery_code, code: code, continue_to: destination)
  end

  defp changeset_error(changeset) do
    Enum.map_join(changeset.errors, " ", fn {field, {message, opts}} ->
      message =
        Enum.reduce(opts, message, fn {key, value}, text ->
          String.replace(text, "%{#{key}}", to_string(value))
        end)

      label =
        case field do
          :password_confirmation -> "Password confirmation"
          :username -> "Username"
          _ -> "Password"
        end

      "#{label} #{message}."
    end)
  end

  def sign_out(conn, _), do: conn |> Auth.log_out() |> redirect(to: "/")

  def show(conn, params) do
    requested_page = if is_binary(params["page"]), do: Integer.parse(params["page"]), else: :error

    page_number =
      case requested_page do
        {n, ""} when n > 0 and n <= 100_000 -> n
        _ -> 1
      end

    games = Games.history(conn.assigns.current_user.id, page_number)

    page(conn,
      mode: :account,
      progress: conn.assigns.current_progress,
      games: Enum.take(games, 20),
      page_number: page_number,
      more: length(games) > 20,
      unclaimed: Games.unclaimed_count(get_session(conn, :owner))
    )
  end

  def claim(conn, _) do
    count = Games.claim(get_session(conn, :owner), conn.assigns.current_user.id)

    conn
    |> put_flash(
      :info,
      "#{count} #{if count == 1, do: "game", else: "games"} added to your account."
    )
    |> redirect(to: "/account")
  end

  defp page(conn, assigns) do
    conn
    |> put_view(html: ScripguessrWeb.AccountHTML)
    |> put_layout(html: {ScripguessrWeb.Layouts, :app})
    |> render(:page, assigns)
  end
end
