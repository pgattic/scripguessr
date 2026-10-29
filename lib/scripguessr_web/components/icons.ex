defmodule ScripguessrWeb.Icons do
  use Phoenix.Component

  @paths %{
    "flame" => ["M12 3c1 5 6 6 6 12a6 6 0 0 1-12 0c0-3 2-5 3-6 0 3 1 4 2 4 2-2 3-5 1-10Z"],
    "close" => ["m6 6 12 12M6 18 18 6"],
    "copy" => ["M9 9h12v12H9z", "M15 9V3H3v12h6"],
    "book" => ["M12 5v16", "M12 5C9 3 5 3 2 4v15c3-1 7-1 10 2 3-3 7-3 10-2V4c-3-1-7-1-10 1Z"],
    "bookmark" => ["M6 3h12v18l-6-4-6 4z"],
    "down" => ["M12 4v16m-6-6 6 6 6-6"],
    "qr" => [
      "M3 3h7v7H3zM14 3h7v7h-7zM3 14h7v7H3z",
      "M6.5 6.5h.01M17.5 6.5h.01M6.5 17.5h.01",
      "M14 14h3v3h-3zM21 14v3M14 21h3M20 20h1v1h-1z"
    ]
  }

  attr :name, :string, required: true
  attr :filled, :boolean, default: false

  def icon(assigns) do
    assigns = assign(assigns, :paths, Map.fetch!(@paths, assigns.name))

    ~H"""
    <svg
      class="ui-icon"
      viewBox="0 0 24 24"
      width="20"
      height="20"
      fill={if @filled, do: "currentColor", else: "none"}
      stroke="currentColor"
      stroke-width="1.7"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      <path :for={path <- @paths} d={path} />
    </svg>
    """
  end

  attr :name, :string, required: true
  attr :label, :string, required: true
  attr :rest, :global, include: ~w(disabled autofocus)

  def icon_button(assigns) do
    ~H"""
    <button
      type="button"
      class="icon-button"
      aria-label={@label}
      data-tooltip={@label}
      {@rest}
    >
      <.icon name={@name} />
    </button>
    """
  end
end
