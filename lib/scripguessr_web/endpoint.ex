defmodule ScripguessrWeb.Endpoint do
  use Phoenix.Endpoint, otp_app: :scripguessr

  @session_options [
    store: :cookie,
    key: "_scripguessr",
    signing_salt: "browser-owner-v1",
    same_site: "Lax",
    http_only: true,
    max_age: 2_592_000
  ]
  socket "/live", Phoenix.LiveView.Socket,
    websocket: [connect_info: [session: @session_options]],
    longpoll: false

  plug Plug.Static, at: "/", from: :scripguessr, gzip: false, only: ~w(assets favicon.svg)

  if code_reloading? do
    plug Phoenix.CodeReloader
  end

  plug Plug.RequestId
  plug Plug.Telemetry, event_prefix: [:phoenix, :endpoint]

  plug Plug.Parsers,
    parsers: [:urlencoded, :multipart, :json],
    pass: ["*/*"],
    json_decoder: Jason,
    length: 64_000

  plug Plug.MethodOverride
  plug Plug.Head
  plug Plug.Session, @session_options
  plug ScripguessrWeb.Router
end
