defmodule ScripguessrWeb.MatchLive do
  use ScripguessrWeb, :live_view
  import ScripguessrWeb.GameComponents, only: [picker: 1, verse_panel: 1, round_feedback: 1]
  import ScripguessrWeb.ChapterReader, only: [chapter_reader: 1]
  import ScripguessrWeb.SavedVerseComponents, only: [save_button: 1]
  alias Scripguessr.{Matches, SavedVerses}
  alias Scripguessr.Matches.Identity
  alias ScripguessrWeb.{GameComponents, MatchPresence}

  @impl true
  def mount(_, session, socket) do
    socket =
      if socket.assigns.current_user do
        token = session["account_token"]

        attach_hook(socket, :match_session, :handle_info, fn
          {:match_updated, _}, socket ->
            if Scripguessr.Accounts.user_for_session(token),
              do: {:cont, socket},
              else: {:halt, redirect(socket, to: "/account/sign-in")}

          _, socket ->
            {:cont, socket}
        end)
      else
        socket
      end

    {:ok,
     assign(socket,
       identity:
         Identity.new(
           socket.assigns.current_user && socket.assigns.current_user.id,
           session["owner"]
         ),
       guest_name: "",
       room: nil,
       code: nil,
       unavailable: false,
       recent: [],
       preparing: false,
       error: nil,
       book: "",
       chapter: "",
       step: :book,
       topic: nil,
       presence_topic: nil,
       online: [],
       reading: nil,
       reader: nil,
       reader_error: nil,
       saved_verses: SavedVerses.keys(socket.assigns.current_user),
       save_error: nil,
       now: DateTime.utc_now()
     )}
  end

  @impl true
  def handle_params(params, _, socket) do
    socket = socket |> detach() |> close_reader()

    case params do
      %{"code" => code} ->
        {:noreply, socket |> assign(code: code, error: nil) |> refresh()}

      _ ->
        {:noreply,
         assign(socket,
           code: nil,
           room: nil,
           unavailable: false,
           recent: Matches.recent(socket.assigns.identity),
           error: nil
         )}
    end
  end

  @impl true
  def handle_event("create", _, %{assigns: %{preparing: true}} = socket), do: {:noreply, socket}

  def handle_event("create", params, socket) do
    name = guest_name(params, socket)
    actor = socket.assigns.identity

    {:noreply,
     socket
     |> assign(preparing: true, error: nil, guest_name: name)
     |> start_async(:create, fn -> Matches.create(actor, 2, name) end)}
  end

  def handle_event("join", params, socket) do
    name = guest_name(params, socket)
    socket = assign(socket, guest_name: name)
    act(socket, fn -> Matches.join(socket.assigns.code, socket.assigns.identity, name) end)
  end

  def handle_event("ready", %{"ready" => value}, socket) when value in ["true", "false"],
    do:
      act(socket, fn ->
        Matches.ready(socket.assigns.code, socket.assigns.identity, value == "true")
      end)

  def handle_event("start", _, socket),
    do: act(socket, fn -> Matches.start(socket.assigns.code, socket.assigns.identity) end)

  def handle_event("advance", %{"round" => value}, socket) when is_binary(value) do
    case Integer.parse(value) do
      {index, ""} ->
        act(socket, fn -> Matches.advance(socket.assigns.code, socket.assigns.identity, index) end)

      _ ->
        {:noreply, socket}
    end
  end

  def handle_event("leave", _, socket) do
    case Matches.leave(socket.assigns.code, socket.assigns.identity) do
      {:ok, _} -> {:noreply, push_navigate(socket, to: "/matches")}
      {:error, reason} -> {:noreply, assign(refresh(socket), :error, error_message(reason))}
    end
  end

  def handle_event("choose-book", %{"book" => book}, socket) do
    if editable?(socket) and Enum.any?(socket.assigns.room.chapters, &(&1["book"] == book)) do
      case GameComponents.chapter_numbers(socket.assigns.room.chapters, book) do
        [number] ->
          {:noreply,
           assign(socket, book: book, chapter: to_string(number), step: :ready, error: nil)}

        _ ->
          {:noreply, assign(socket, book: book, chapter: "", step: :chapter, error: nil)}
      end
    else
      {:noreply, socket}
    end
  end

  def handle_event("choose-chapter", %{"chapter" => chapter}, socket) do
    if editable?(socket) and
         chapter in Enum.map(
           GameComponents.chapter_numbers(socket.assigns.room.chapters, socket.assigns.book),
           &to_string/1
         ),
       do: {:noreply, assign(socket, chapter: chapter, step: :ready, error: nil)},
       else: {:noreply, socket}
  end

  def handle_event("edit-book", _, socket) do
    if editable?(socket) and socket.assigns.book != "" do
      step =
        if length(
             GameComponents.chapter_numbers(socket.assigns.room.chapters, socket.assigns.book)
           ) ==
             1, do: :ready, else: :chapter

      {:noreply, assign(socket, step: step)}
    else
      {:noreply, socket}
    end
  end

  def handle_event("edit-chapter", _, socket),
    do:
      {:noreply,
       if(editable?(socket) and socket.assigns.chapter != "",
         do: assign(socket, step: :ready),
         else: socket
       )}

  def handle_event("clear-guess", _, socket),
    do: {:noreply, if(editable?(socket), do: reset_picker(socket), else: socket)}

  def handle_event(
        "guess",
        %{"guess" => %{"round" => index, "book" => book, "chapter" => chapter}},
        socket
      )
      when is_binary(index) do
    case Integer.parse(index) do
      {index, ""} ->
        act(socket, fn ->
          Matches.submit(socket.assigns.code, socket.assigns.identity, index, book, chapter)
        end)

      _ ->
        {:noreply, assign(socket, :error, "Choose a valid book and chapter.")}
    end
  end

  def handle_event("read-chapter", %{"round" => value}, socket) when is_binary(value) do
    case Integer.parse(value) do
      {index, ""} when index in 0..4 -> {:noreply, open_chapter(socket, index)}
      _ -> {:noreply, socket}
    end
  end

  def handle_event("close-chapter", _, socket), do: {:noreply, close_reader(socket)}

  def handle_event("retry-chapter", _, %{assigns: %{reading: %{round: index}}} = socket),
    do: {:noreply, open_chapter(socket, index)}

  def handle_event("save-verse", %{"source" => "round", "index" => value}, socket)
      when is_binary(value) do
    socket = refresh(socket)

    with {index, ""} when index in 0..4 <- Integer.parse(value),
         result when not is_nil(result) <- readable_result(socket.assigns.room, index) do
      {:noreply, save_verse(socket, result_verse(result), socket.assigns.room.release)}
    else
      _ -> {:noreply, socket}
    end
  end

  def handle_event("save-verse", %{"source" => "reader", "verse" => value}, socket)
      when is_binary(value) do
    socket = refresh(socket)

    with {number, ""} when number > 0 <- Integer.parse(value),
         reading when not is_nil(reading) <- socket.assigns.reading,
         verses when is_list(verses) <- socket.assigns.reader,
         verse when not is_nil(verse) <- Enum.find(verses, &(&1["verse"] == number)) do
      verse = Map.merge(verse, %{"book" => reading.book, "chapter" => reading.chapter})
      {:noreply, save_verse(socket, verse, reading.release)}
    else
      _ -> {:noreply, socket}
    end
  end

  def handle_event(_, _, socket), do: {:noreply, socket}

  @impl true
  def handle_async(:create, {:ok, {:ok, code}}, socket),
    do: {:noreply, socket |> assign(preparing: false) |> push_navigate(to: "/matches/#{code}")}

  def handle_async(:create, {:ok, {:error, reason}}, socket)
      when reason in [:invalid_name, :identity_required] do
    {:noreply, assign(socket, preparing: false, error: error_message(reason))}
  end

  def handle_async(:create, _, socket),
    do:
      {:noreply,
       assign(socket, preparing: false, error: "We couldn’t load the verses. Please try again.")}

  def handle_async(:chapter, result, socket) do
    # A host may advance while the upstream request is in flight. Recheck the
    # persisted phase before delivering chapter text to this player.
    socket = refresh(socket)

    if socket.assigns.reading do
      case result do
        {:ok, {:ok, verses}} ->
          {:noreply, assign(socket, reader: verses, reader_error: nil)}

        {:ok, {:error, :release_changed}} ->
          {:noreply,
           assign(socket,
             reader_error: "The corpus has changed since this match. This chapter is unavailable."
           )}

        _ ->
          {:noreply, assign(socket, reader_error: "Couldn’t load this chapter. Try again.")}
      end
    else
      {:noreply, socket}
    end
  end

  @impl true
  def handle_info({:match_updated, id}, socket) do
    if socket.assigns.room && socket.assigns.room.id == id,
      do: {:noreply, refresh(socket)},
      else: {:noreply, socket}
  end

  def handle_info(%Phoenix.Socket.Broadcast{event: "presence_diff"}, socket),
    do: {:noreply, assign_online(socket)}

  defp refresh(%{assigns: %{code: nil}} = socket), do: socket

  defp refresh(socket) do
    case Matches.fetch(socket.assigns.code, socket.assigns.identity) do
      {:ok, room} ->
        old = socket.assigns.room

        socket =
          if old && (old.round != room.round or old.status != room.status),
            do: socket |> reset_picker() |> close_reader(),
            else: socket

        socket
        |> assign(room: room, unavailable: false, now: DateTime.utc_now())
        |> attach()
        |> assign_online()

      _ ->
        socket |> close_reader() |> assign(room: nil, unavailable: true)
    end
  end

  defp attach(socket) do
    room = socket.assigns.room
    topic = Matches.topic(room.id)

    socket =
      if connected?(socket) and socket.assigns.topic != topic do
        Phoenix.PubSub.subscribe(Scripguessr.PubSub, topic)
        assign(socket, :topic, topic)
      else
        socket
      end

    if connected?(socket) and room.joined? and socket.assigns.presence_topic != topic do
      {:ok, _} = MatchPresence.track(self(), topic, room.player_id, %{})
      assign(socket, :presence_topic, topic)
    else
      socket
    end
  end

  defp detach(socket) do
    if socket.assigns.presence_topic,
      do:
        MatchPresence.untrack(
          self(),
          socket.assigns.presence_topic,
          socket.assigns.room.player_id
        )

    if socket.assigns.topic,
      do: Phoenix.PubSub.unsubscribe(Scripguessr.PubSub, socket.assigns.topic)

    assign(socket, topic: nil, presence_topic: nil, online: [])
  end

  defp assign_online(%{assigns: %{topic: nil}} = socket), do: socket

  defp assign_online(socket),
    do: assign(socket, :online, MatchPresence.list(socket.assigns.topic) |> Map.keys())

  defp act(socket, action) do
    case action.() do
      {:ok, _} -> {:noreply, assign(refresh(socket), :error, nil)}
      {:error, reason} -> {:noreply, assign(refresh(socket), :error, error_message(reason))}
    end
  end

  defp guest_name(%{"guest" => %{"name" => name}}, _) when is_binary(name), do: name
  defp guest_name(_, socket), do: socket.assigns.guest_name
  defp user_id(socket), do: socket.assigns.current_user && socket.assigns.current_user.id

  defp readable_result(%{joined?: true, status: "reveal", round: index, result: result}, index),
    do: result

  defp readable_result(%{joined?: true, status: "finished", results: results}, index),
    do: Enum.find(results, &(&1.round == index))

  defp readable_result(_, _), do: nil

  defp open_chapter(socket, index) do
    socket = refresh(socket)

    case readable_result(socket.assigns.room, index) do
      nil ->
        close_reader(socket)

      result ->
        release = socket.assigns.room.release

        socket
        |> cancel_async(:chapter)
        |> assign(
          reading: Map.put(result, :release, release),
          reader: nil,
          reader_error: nil,
          save_error: nil
        )
        |> start_async(:chapter, fn ->
          Scripguessr.Corpus.Chapter.read(result.book, result.chapter, release)
        end)
    end
  end

  defp close_reader(socket),
    do:
      socket
      |> cancel_async(:chapter)
      |> assign(reading: nil, reader: nil, reader_error: nil, save_error: nil)

  defp result_verse(result),
    do: %{
      "book" => result.book,
      "chapter" => result.chapter,
      "verse" => result.verse,
      "text" => result.text
    }

  defp result_reference(result), do: "#{result.book} #{result.chapter}:#{result.verse}"
  defp guess_label(%{book: nil}), do: "No answer"
  defp guess_label(guess), do: "#{guess.book} #{guess.chapter}"

  defp save_verse(%{assigns: %{current_user: nil}} = socket, _, _),
    do: assign(socket, save_error: "Sign in to save verses.")

  defp save_verse(socket, verse, release) do
    key = SavedVerses.key(verse)
    saved? = !MapSet.member?(socket.assigns.saved_verses, key)

    case SavedVerses.set_saved(user_id(socket), verse, release, saved?) do
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
        assign(socket, save_error: "Couldn’t update this saved verse. Please try again.")
    end
  end

  defp editable?(socket),
    do:
      socket.assigns.room && socket.assigns.room.joined? && socket.assigns.room.status == "round" &&
        !socket.assigns.room.own_guess

  defp reset_picker(socket), do: assign(socket, book: "", chapter: "", step: :book, error: nil)

  defp sign_in_path(code),
    do:
      "/account/sign-in?" <>
        URI.encode_query(%{return_to: if(code, do: "/matches/#{code}", else: "/matches")})

  defp seconds_left(room, now),
    do:
      if(room.deadline,
        do: max(0, ceil(DateTime.diff(room.deadline, now, :millisecond) / 1000)),
        else: 0
      )

  defp own_player(room), do: Enum.find(room.players, &(&1.id == room.player_id))

  defp winner_text(room, player_id) do
    best = room.players |> Enum.map(& &1.score) |> Enum.max()
    winners = Enum.filter(room.players, &(&1.score == best))

    cond do
      length(winners) > 1 -> "It’s a tie!"
      hd(winners).id == player_id -> "You won!"
      true -> "#{hd(winners).username} wins"
    end
  end

  defp error_message(:invalid_name),
    do: "Choose a name between 1 and 32 characters, without control characters."

  defp error_message(:name_taken), do: "That name is already in this room. Choose another."
  defp error_message(:identity_required), do: "Refresh the page to start a browser session."
  defp error_message(:full), do: "This room is full."
  defp error_message(:not_ready), do: "Everyone needs to join and be ready first."
  defp error_message(:host_only), do: "Only the host can start or advance the match."
  defp error_message(:not_a_player), do: "Join the room to play."
  defp error_message(:already_started), do: "This match has already started or ended."
  defp error_message(:invalid_guess), do: "Choose a valid book and chapter."
  defp error_message(:stale_round), do: "The round has ended. Your saved result is shown below."
  defp error_message(_), do: "This room is unavailable."

  @impl true
  def render(assigns) do
    assigns =
      assign(assigns,
        player:
          if(assigns.room,
            do: own_player(assigns.room)
          )
      )

    match(assigns)
  end

  embed_templates "match.html"
end
