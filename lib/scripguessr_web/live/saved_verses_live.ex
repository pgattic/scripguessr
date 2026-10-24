defmodule ScripguessrWeb.SavedVersesLive do
  use ScripguessrWeb, :live_view
  import ScripguessrWeb.Layouts, only: [personal_nav: 1]
  alias Scripguessr.SavedVerses

  @impl true
  def mount(_, _, socket) do
    if socket.assigns.current_user do
      {:ok, socket}
    else
      {:ok, redirect(socket, to: "/account/sign-in")}
    end
  end

  @impl true
  def handle_params(params, _, socket) do
    number =
      case params["page"] do
        value when is_binary(value) ->
          case Integer.parse(value) do
            {n, ""} when n > 0 and n <= 100_000 -> n
            _ -> 1
          end

        _ ->
          1
      end

    {:noreply,
     assign(socket, :saved_page, SavedVerses.page(socket.assigns.current_user.id, number))}
  end

  @impl true
  def handle_event("remove", %{"id" => id}, socket) do
    SavedVerses.remove_by_id(socket.assigns.current_user.id, id)
    page = SavedVerses.page(socket.assigns.current_user.id, socket.assigns.saved_page.number)

    if page.entries == [] and page.number > 1 do
      {:noreply, push_patch(socket, to: "/saved-verses?page=#{page.number - 1}")}
    else
      {:noreply, assign(socket, :saved_page, page)}
    end
  end

  @impl true
  def render(assigns) do
    ~H"""
    <section class="saved-page">
      <h1>Saved verses</h1>
      <.personal_nav active={:saved} />
      <div :if={@saved_page.entries == [] && @saved_page.number == 1} class="panel saved-empty">
        <p>No saved verses yet.</p>
        <p class="muted">Save a verse after a round or while reading a chapter.</p>
        <a href="/atlas" class="text-button">Explore the Atlas →</a>
      </div>
      <div id="saved-verses" class="saved-list">
        <article :for={verse <- @saved_page.entries} id={"saved-#{verse.id}"} class="panel saved-card">
          <div class="saved-card-heading">
            <a href={"/atlas?" <> URI.encode_query(%{book: verse.book, chapter: verse.chapter})}>
              {verse.book} {verse.chapter}:{verse.verse}
            </a>
            <button
              type="button"
              class="text-button"
              phx-click="remove"
              phx-value-id={verse.id}
              aria-label={"Unsave #{verse.book} #{verse.chapter}:#{verse.verse}"}
            >Unsave</button>
          </div>
          <p class="saved-text">{verse.text}</p>
        </article>
      </div>
      <p :if={@saved_page.entries == [] && @saved_page.number > 1} class="muted">
        No verses on this page.
      </p>
      <nav
        :if={@saved_page.number > 1 || @saved_page.more?}
        class="saved-pagination"
        aria-label="Saved verses pages"
      >
        <.link
          :if={@saved_page.number > 1}
          patch={"/saved-verses?page=#{@saved_page.number - 1}"}
          class="crumb"
        >← Newer</.link>
        <.link
          :if={@saved_page.more?}
          patch={"/saved-verses?page=#{@saved_page.number + 1}"}
          class="crumb"
        >Older →</.link>
      </nav>
    </section>
    """
  end
end
