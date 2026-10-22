defmodule ScripguessrWeb do
  def live_view do
    quote do
      use Phoenix.LiveView, layout: {ScripguessrWeb.Layouts, :app}
      import Phoenix.Component
      alias Phoenix.LiveView.JS
    end
  end

  def html do
    quote do
      use Phoenix.Component
      import Phoenix.Controller, only: [get_csrf_token: 0]
    end
  end

  defmacro __using__(which), do: apply(__MODULE__, which, [])
end
