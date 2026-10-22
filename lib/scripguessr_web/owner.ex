defmodule ScripguessrWeb.Owner do
  import Plug.Conn
  def init(opts), do: opts

  def call(conn, _opts) do
    if get_session(conn, :owner) do
      conn
    else
      put_session(conn, :owner, Base.url_encode64(:crypto.strong_rand_bytes(32), padding: false))
    end
  end
end
