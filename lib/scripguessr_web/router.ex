defmodule ScripguessrWeb.Router do
  use Phoenix.Router
  import Phoenix.LiveView.Router

  pipeline :browser do
    plug :accepts, ["html"]
    plug :fetch_session
    plug :fetch_live_flash
    plug :put_root_layout, html: {ScripguessrWeb.Layouts, :root}
    plug :protect_from_forgery
    plug :put_secure_browser_headers
    plug ScripguessrWeb.Owner
  end

  get "/healthz", ScripguessrWeb.HealthController, :live
  get "/readyz", ScripguessrWeb.HealthController, :ready

  scope "/", ScripguessrWeb do
    pipe_through :browser
    live "/", QuizLive, :home
    live "/quiz/:id", QuizLive, :quiz
  end
end
