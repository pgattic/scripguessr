defmodule ScripguessrWeb.GameLive do
  use ScripguessrWeb, :live_view
  import ScripguessrWeb.ChapterReader, only: [chapter_reader: 1]
  alias Scripguessr.SavedVerses
  import ScripguessrWeb.SavedVerseComponents, only: [save_button: 1]
  alias Scripguessr.{Games, Progression}
  import ScripguessrWeb.ProgressionComponents
  alias Scripguessr.Games.Rules

  @impl true
  def mount(_params, session, socket) do
    {:ok,
     assign(socket,
       owner: session["owner"],
       game: nil,
       unavailable: false,
       preparing: false,
       error: nil,
       book: "",
       chapter: "",
       step: :book,
       reading: nil,
       reader: nil,
       reader_error: nil,
       progress: nil,
       reward: nil,
       saved_verses: SavedVerses.keys(socket.assigns.current_user),
       save_error: nil
     )}
  end

  @impl true
  def handle_params(params, _uri, socket) do
    case params do
      %{"id" => id} ->
        case Games.fetch(id, socket.assigns.owner, user_id(socket)) do
          {:ok, game} ->
            {:noreply,
             socket
             |> assign(
               game: game,
               unavailable: false,
               error: nil,
               book: "",
               chapter: "",
               step: :book
             )
             |> assign_progress(game)}

          {:error, _} ->
            {:noreply, assign(socket, game: nil, unavailable: true)}
        end

      _ ->
        {:noreply, assign(socket, game: nil, unavailable: false)}
    end
  end

  @impl true
  def handle_event("start", _, %{assigns: %{preparing: true}} = socket), do: {:noreply, socket}

  def handle_event("start", _, socket) do
    owner = socket.assigns.owner
    user_id = user_id(socket)

    {:noreply,
     socket
     |> assign(preparing: true, error: nil)
     |> start_async(:prepare, fn -> Games.create(owner, user_id) end)}
  end

  def handle_event("choose-book", %{"book" => book}, socket) do
    if editable?(socket) and book in books(socket.assigns.game.snapshot) do
      {:noreply, assign(socket, book: book, chapter: "", step: :chapter, error: nil)}
    else
      {:noreply, socket}
    end
  end

  def handle_event("choose-chapter", %{"chapter" => chapter}, socket) do
    if editable?(socket) and
         chapter in Enum.map(
           chapters(socket.assigns.game.snapshot, socket.assigns.book),
           &to_string/1
         ) do
      {:noreply, assign(socket, chapter: chapter, step: :ready, error: nil)}
    else
      {:noreply, socket}
    end
  end

  def handle_event("edit-book", _, socket) do
    if editable?(socket) and socket.assigns.book != "",
      do: {:noreply, assign(socket, step: :chapter)},
      else: {:noreply, socket}
  end

  def handle_event("edit-chapter", _, socket) do
    if editable?(socket) and socket.assigns.chapter != "",
      do: {:noreply, assign(socket, step: :ready)},
      else: {:noreply, socket}
  end

  def handle_event("clear-guess", _, socket) do
    if editable?(socket),
      do: {:noreply, assign(socket, book: "", chapter: "", step: :book, error: nil)},
      else: {:noreply, socket}
  end

  def handle_event(
        "guess",
        %{"guess" => %{"book" => book, "chapter" => chapter, "round" => index}},
        %{assigns: %{game: %{id: id}}} = socket
      )
      when is_binary(index) do
    with {index, ""} <- Integer.parse(index) do
      update(
        socket,
        Games.submit(id, socket.assigns.owner, index, book, chapter, user_id(socket))
      )
    else
      _ -> {:noreply, assign(socket, error: "Choose a valid book and chapter.")}
    end
  end

  def handle_event("advance", %{"round" => index}, %{assigns: %{game: %{id: id}}} = socket)
      when is_binary(index) do
    with {index, ""} <- Integer.parse(index) do
      update(socket, Games.advance(id, socket.assigns.owner, index, user_id(socket)))
    else
      _ -> {:noreply, socket}
    end
  end

  def handle_event(
        "read-chapter",
        %{"round" => index},
        %{assigns: %{game: %{snapshot: snapshot}}} = socket
      )
      when is_binary(index) do
    with {index, ""} when index >= 0 <- Integer.parse(index),
         %{"guess" => guess} = round when not is_nil(guess) <- Enum.at(snapshot["rounds"], index) do
      {:noreply, load_chapter(socket, round)}
    else
      _ -> {:noreply, socket}
    end
  end

  def handle_event("save-verse", %{"source" => "round", "index" => value}, socket)
      when is_binary(value) do
    with {index, ""} when index >= 0 <- Integer.parse(value),
         %{snapshot: snapshot} <- socket.assigns.game,
         %{"guess" => guess} = verse when not is_nil(guess) <- Enum.at(snapshot["rounds"], index) do
      {:noreply, save_verse(socket, verse, snapshot["release"])}
    else
      _ -> {:noreply, socket}
    end
  end

  def handle_event("save-verse", %{"source" => "reader", "verse" => value}, socket)
      when is_binary(value) do
    with {number, ""} when number > 0 <- Integer.parse(value),
         reading when not is_nil(reading) <- socket.assigns.reading,
         verses when is_list(verses) <- socket.assigns.reader,
         verse when not is_nil(verse) <- Enum.find(verses, &(&1["verse"] == number)) do
      verse = Map.merge(verse, Map.take(reading, ["book", "chapter"]))

      {:noreply, save_verse(socket, verse, socket.assigns.game.snapshot["release"])}
    else
      _ -> {:noreply, socket}
    end
  end

  def handle_event("close-chapter", _, socket), do: {:noreply, close_reader(socket)}

  def handle_event("retry-chapter", _, %{assigns: %{reading: round}} = socket)
      when not is_nil(round),
      do: {:noreply, load_chapter(socket, round)}

  def handle_event(_event, _params, socket), do: {:noreply, socket}

  @impl true
  def handle_async(:prepare, {:ok, {:ok, game}}, socket) do
    {:noreply, socket |> assign(preparing: false) |> push_navigate(to: "/game/#{game.id}")}
  end

  def handle_async(:prepare, _result, socket) do
    {:noreply,
     assign(socket,
       preparing: false,
       error: "We couldn’t load your verses. Please try again."
     )}
  end

  def handle_async(:chapter, {:ok, {:ok, verses}}, socket),
    do: {:noreply, assign(socket, reader: verses, reader_error: nil)}

  def handle_async(:chapter, {:ok, {:error, :release_changed}}, socket),
    do:
      {:noreply,
       assign(socket,
         reader_error:
           "The corpus has changed since this game. This chapter is unavailable for this session."
       )}

  def handle_async(:chapter, _result, socket),
    do: {:noreply, assign(socket, reader_error: "Couldn’t load this chapter. Try again.")}

  defp load_chapter(socket, round) do
    release = socket.assigns.game.snapshot["release"]

    socket
    |> cancel_async(:chapter)
    |> assign(reading: round, reader: nil, reader_error: nil)
    |> start_async(:chapter, fn ->
      Scripguessr.Corpus.Chapter.read(round["book"], round["chapter"], release)
    end)
  end

  defp close_reader(socket) do
    socket
    |> cancel_async(:chapter)
    |> assign(reading: nil, reader: nil, reader_error: nil)
  end

  defp update(socket, {:ok, game}),
    do:
      {:noreply,
       socket
       |> close_reader()
       |> assign(game: game, book: "", chapter: "", step: :book, error: nil)
       |> assign_progress(game)}

  defp update(socket, {:error, :invalid_guess}),
    do: {:noreply, assign(socket, error: "Choose a valid book and chapter.")}

  defp update(socket, {:error, _}) do
    case Games.fetch(socket.assigns.game.id, socket.assigns.owner, user_id(socket)) do
      {:ok, game} ->
        {:noreply,
         socket
         |> assign(
           game: game,
           error: "This round changed in another tab. Your saved progress is shown below."
         )
         |> assign_progress(game)}

      _ ->
        {:noreply, assign(socket, game: nil, unavailable: true)}
    end
  end

  defp assign_progress(socket, game) do
    if game.snapshot["finished"] and socket.assigns.current_user do
      progress = Progression.for_user(socket.assigns.current_user.id)
      assign(socket, progress: progress, reward: Progression.for_game(game.id))
    else
      assign(socket, progress: nil, reward: nil)
    end
  end

  defp user_id(socket), do: socket.assigns.current_user && socket.assigns.current_user.id

  defp reference(round), do: "#{round["book"]} #{round["chapter"]}:#{round["verse"]}"
  defp books(snapshot), do: snapshot["chapters"] |> Enum.map(& &1["book"]) |> Enum.uniq()

  defp chapters(snapshot, book),
    do: snapshot["chapters"] |> Enum.filter(&(&1["book"] == book)) |> Enum.map(& &1["chapter"])

  defp editable?(%{assigns: %{game: %{snapshot: snapshot}}}) do
    !snapshot["finished"] and is_nil(Enum.at(snapshot["rounds"], snapshot["current"])["guess"])
  end

  defp editable?(_), do: false

  defp distance_label(0), do: "Exact chapter"
  defp distance_label(1), do: "Off by 1 chapter"
  defp distance_label(n), do: "Off by #{n} chapters"

  defp save_verse(socket, verse, release) do
    if user = socket.assigns.current_user do
      key = SavedVerses.key(verse)
      saved? = !MapSet.member?(socket.assigns.saved_verses, key)

      case SavedVerses.set_saved(user.id, verse, release, saved?) do
        {:ok, true} ->
          assign(socket,
            saved_verses: MapSet.put(socket.assigns.saved_verses, key),
            save_error: nil
          )

        {:ok, false} ->
          assign(socket,
            saved_verses: MapSet.delete(socket.assigns.saved_verses, key),
            save_error: nil
          )

        {:error, _} ->
          assign(socket, :save_error, "Couldn’t update this saved verse. Please try again.")
      end
    else
      socket
    end
  end

  @impl true
  def render(assigns) do
    snapshot = if assigns.game, do: assigns.game.snapshot

    assigns =
      assign(assigns,
        snapshot: snapshot,
        xp_reward: Progression.Rules.reward(snapshot),
        round: if(snapshot, do: Enum.at(snapshot["rounds"], snapshot["current"]))
      )

    game(assigns)
  end

  embed_templates "game.html"
end
