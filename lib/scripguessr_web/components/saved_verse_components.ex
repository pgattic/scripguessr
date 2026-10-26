defmodule ScripguessrWeb.SavedVerseComponents do
  use Phoenix.Component
  import ScripguessrWeb.Icons
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
      class={["text-button save-verse icon-label", @source == "reader" && "reader-save-verse"]}
      phx-click="save-verse"
      phx-value-source={@source}
      phx-value-index={@index}
      phx-value-verse={@verse["verse"]}
      aria-pressed={to_string(@is_saved)}
      aria-label={if @is_saved, do: "Unsave verse", else: "Save verse"}
      title={if @is_saved, do: "Unsave verse", else: "Save verse"}
    >
      <.icon name="bookmark" filled={@is_saved} />
      <span :if={@source != "reader"}>{if @is_saved, do: "Saved", else: "Save verse"}</span>
    </button>
    <a :if={!@user && @source == "round"} class="text-button save-verse" href="/account/sign-in">Sign in to save</a>
    """
  end
end
