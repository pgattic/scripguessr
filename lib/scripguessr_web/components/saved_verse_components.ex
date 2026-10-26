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
      class={[
        "save-verse",
        if(@source == "reader", do: "text-button reader-save-verse", else: "icon-button")
      ]}
      phx-click="save-verse"
      phx-value-source={@source}
      phx-value-index={@index}
      phx-value-verse={@verse["verse"]}
      aria-pressed={to_string(@is_saved)}
      aria-label={if @is_saved, do: "Unsave verse", else: "Save verse"}
      title={if @source == "reader", do: if(@is_saved, do: "Unsave verse", else: "Save verse")}
      data-tooltip={if @is_saved, do: "Unsave verse", else: "Save verse"}
    >
      <.icon name="bookmark" filled={@is_saved} />
    </button>
    <a
      :if={!@user && @source == "round"}
      class="icon-button save-verse"
      href="/account/sign-in"
      aria-label="Sign in to save verse"
      data-tooltip="Sign in to save verse"
    ><.icon name="bookmark" /></a>
    """
  end
end
