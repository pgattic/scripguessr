defmodule ScripguessrWeb.SavedVerseComponents do
  use Phoenix.Component
  alias Scripguessr.SavedVerses

  attr :verse, :map, required: true
  attr :saved, :any, required: true
  attr :user, :any, required: true
  attr :source, :string, default: "reader"
  attr :index, :integer, default: nil

  def save_button(assigns) do
    assigns =
      assign(assigns, :is_saved, MapSet.member?(assigns.saved, SavedVerses.key(assigns.verse)))

    ~H"""
    <button
      :if={@user}
      type="button"
      class={["text-button save-verse", @source == "reader" && "reader-save-verse"]}
      phx-click="save-verse"
      phx-value-source={@source}
      phx-value-index={@index}
      phx-value-verse={@verse["verse"]}
      aria-pressed={to_string(@is_saved)}
      aria-label={if @is_saved, do: "Unsave verse", else: "Save verse"}
      title={if @is_saved, do: "Unsave verse", else: "Save verse"}
    >
      <svg
        :if={@source == "reader"}
        viewBox="0 0 24 24"
        width="18"
        height="18"
        aria-hidden="true"
        fill={if @is_saved, do: "currentColor", else: "none"}
        stroke="currentColor"
        stroke-width="1.6"
      >
        <path d="M6 3h12v18l-6-4-6 4z" stroke-linejoin="round" />
      </svg>
      <span :if={@source != "reader"}>{if @is_saved, do: "Saved ✓", else: "Save verse"}</span>
    </button>
    <a :if={!@user && @source == "round"} class="text-button save-verse" href="/account/sign-in">Sign in to save</a>
    """
  end
end
