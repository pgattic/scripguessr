defmodule ScripguessrWeb.MatchLive do
  use ScripguessrWeb, :live_view
  import ScripguessrWeb.MatchPicker, only: [picker: 1]
  alias Scripguessr.Matches
  alias ScripguessrWeb.{MatchPicker, MatchPresence}

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
       now: DateTime.utc_now()
     )}
  end

  @impl true
  def handle_params(params, _, socket) do
    socket = detach(socket)

    case params do
      %{"code" => code} ->
        {:noreply, socket |> assign(code: code, error: nil) |> refresh()}

      _ ->
        {:noreply,
         assign(socket,
           code: nil,
           room: nil,
           unavailable: false,
           recent: Matches.recent(user_id(socket)),
           error: nil
         )}
    end
  end

  @impl true
  def handle_event("create", _, %{assigns: %{preparing: true}} = socket), do: {:noreply, socket}

  def handle_event("create", _, socket) do
    if user_id = user_id(socket) do
      {:noreply,
       socket
       |> assign(preparing: true, error: nil)
       |> start_async(:create, fn -> Matches.create(user_id) end)}
    else
      {:noreply, redirect(socket, to: sign_in_path(socket.assigns.code))}
    end
  end

  def handle_event("join", _, socket),
    do: act(socket, fn -> Matches.join(socket.assigns.code, user_id(socket)) end)

  def handle_event("ready", %{"ready" => value}, socket) when value in ["true", "false"],
    do:
      act(socket, fn -> Matches.ready(socket.assigns.code, user_id(socket), value == "true") end)

  def handle_event("start", _, socket),
    do: act(socket, fn -> Matches.start(socket.assigns.code, user_id(socket)) end)

  def handle_event("advance", %{"round" => value}, socket) when is_binary(value) do
    case Integer.parse(value) do
      {index, ""} ->
        act(socket, fn -> Matches.advance(socket.assigns.code, user_id(socket), index) end)

      _ ->
        {:noreply, socket}
    end
  end

  def handle_event("leave", _, socket) do
    case Matches.leave(socket.assigns.code, user_id(socket)) do
      {:ok, _} -> {:noreply, push_navigate(socket, to: "/matches")}
      {:error, reason} -> {:noreply, assign(refresh(socket), :error, error_message(reason))}
    end
  end

  def handle_event("choose-book", %{"book" => book}, socket) do
    if editable?(socket) and Enum.any?(socket.assigns.room.chapters, &(&1["book"] == book)) do
      case MatchPicker.chapter_numbers(socket.assigns.room.chapters, book) do
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
           MatchPicker.chapter_numbers(socket.assigns.room.chapters, socket.assigns.book),
           &to_string/1
         ),
       do: {:noreply, assign(socket, chapter: chapter, step: :ready, error: nil)},
       else: {:noreply, socket}
  end

  def handle_event("edit-book", _, socket) do
    if editable?(socket) and socket.assigns.book != "" do
      step =
        if length(MatchPicker.chapter_numbers(socket.assigns.room.chapters, socket.assigns.book)) ==
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
          Matches.submit(socket.assigns.code, user_id(socket), index, book, chapter)
        end)

      _ ->
        {:noreply, assign(socket, :error, "Choose a valid book and chapter.")}
    end
  end

  def handle_event(_, _, socket), do: {:noreply, socket}

  @impl true
  def handle_async(:create, {:ok, {:ok, code}}, socket),
    do: {:noreply, socket |> assign(preparing: false) |> push_navigate(to: "/matches/#{code}")}

  def handle_async(:create, _, socket),
    do:
      {:noreply,
       assign(socket, preparing: false, error: "We couldn’t load the verses. Please try again.")}

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
    case Matches.fetch(socket.assigns.code, user_id(socket)) do
      {:ok, room} ->
        old = socket.assigns.room

        socket =
          if old && (old.round != room.round or old.status != room.status),
            do: reset_picker(socket),
            else: socket

        socket
        |> assign(room: room, unavailable: false, now: DateTime.utc_now())
        |> attach()
        |> assign_online()

      _ ->
        assign(socket, room: nil, unavailable: true)
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
      {:ok, _} = MatchPresence.track(self(), topic, user_id(socket), %{})
      assign(socket, :presence_topic, topic)
    else
      socket
    end
  end

  defp detach(socket) do
    if socket.assigns.presence_topic,
      do: MatchPresence.untrack(self(), socket.assigns.presence_topic, user_id(socket))

    if socket.assigns.topic,
      do: Phoenix.PubSub.unsubscribe(Scripguessr.PubSub, socket.assigns.topic)

    assign(socket, topic: nil, presence_topic: nil, online: [])
  end

  defp assign_online(%{assigns: %{topic: nil}} = socket), do: socket

  defp assign_online(socket),
    do: assign(socket, :online, MatchPresence.list(socket.assigns.topic) |> Map.keys())

  defp act(socket, action) do
    if user_id(socket) do
      case action.() do
        {:ok, _} -> {:noreply, assign(refresh(socket), :error, nil)}
        {:error, reason} -> {:noreply, assign(refresh(socket), :error, error_message(reason))}
      end
    else
      {:noreply, redirect(socket, to: sign_in_path(socket.assigns.code))}
    end
  end

  defp user_id(socket), do: socket.assigns.current_user && socket.assigns.current_user.id

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

  defp own_player(room, user_id), do: Enum.find(room.players, &(&1.user_id == user_id))

  defp winner_text(room, user_id) do
    best = room.players |> Enum.map(& &1.score) |> Enum.max()
    winners = Enum.filter(room.players, &(&1.score == best))

    cond do
      length(winners) > 1 -> "It’s a tie!"
      hd(winners).user_id == user_id -> "You won!"
      true -> "#{hd(winners).username} wins"
    end
  end

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
            do: own_player(assigns.room, assigns.current_user && assigns.current_user.id)
          )
      )

    match(assigns)
  end

  embed_templates "match.html"
end
