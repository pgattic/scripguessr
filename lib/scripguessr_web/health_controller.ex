defmodule ScripguessrWeb.HealthController do
  use Phoenix.Controller, formats: [:html]
  def live(conn, _params), do: Plug.Conn.send_resp(conn, 200, "ok")

  def ready(conn, _params) do
    case Ecto.Adapters.SQL.query(Scripguessr.Repo, "SELECT 1", [], timeout: 2_000) do
      {:ok, _} -> Plug.Conn.send_resp(conn, 200, "ready")
      _ -> Plug.Conn.send_resp(conn, 503, "database unavailable")
    end
  end
end
