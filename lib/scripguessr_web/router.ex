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
    plug ScripguessrWeb.Auth
  end

  get "/healthz", ScripguessrWeb.HealthController, :live
  get "/readyz", ScripguessrWeb.HealthController, :ready

  scope "/", ScripguessrWeb do
    pipe_through :browser
    get "/account/sign-in", AccountController, :new
    post "/account/sign-in", AccountController, :sign_in
    get "/account/register", AccountController, :registration
    post "/account/register", AccountController, :register
    get "/account/recover", AccountController, :recovery
    post "/account/recover", AccountController, :recover
    post "/account/sign-out", AccountController, :sign_out
    get "/account", AccountController, :show
    post "/account/claim-games", AccountController, :claim

    live_session :public, on_mount: [{ScripguessrWeb.Auth, :current_user}] do
      live "/", GameLive, :home
      live "/game/:id", GameLive, :game
      live "/atlas", AtlasLive, :index
    end
  end
end
