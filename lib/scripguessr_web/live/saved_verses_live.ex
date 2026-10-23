defmodule ScripguessrWeb.SavedVersesLive do
  use ScripguessrWeb, :live_view
  alias Scripguessr.SavedVerses

  @impl true
  def mount(_, _, socket) do
    if socket.assigns.current_user do
      {:ok, assign(socket, :verses, SavedVerses.list(socket.assigns.current_user.id))}
    else
      {:ok, redirect(socket, to: "/account/sign-in")}
    end
  end

  @impl true
  def handle_event("remove", %{"id" => id}, socket) do
    SavedVerses.remove_by_id(socket.assigns.current_user.id, id)
    {:noreply, assign(socket, :verses, SavedVerses.list(socket.assigns.current_user.id))}
  end

  @impl true
  def render(assigns) do
    ~H"""
    <section class="saved-page">
      <h1>Saved verses</h1>
      <div :if={@verses == []} class="panel saved-empty">
        <p>No saved verses yet.</p>
        <p class="muted">Save a verse after a round or while reading a chapter.</p>
        <a href="/atlas" class="text-button">Explore the Atlas →</a>
      </div>
      <div id="saved-verses" class="saved-list">
        <article :for={verse <- @verses} id={"saved-#{verse.id}"} class="panel saved-card">
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
    </section>
    """
  end
end
