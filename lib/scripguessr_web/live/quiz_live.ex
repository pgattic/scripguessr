defmodule ScripguessrWeb.QuizLive do
  use ScripguessrWeb, :live_view
  alias Scripguessr.Quizzes
  alias Scripguessr.Quizzes.Rules

  @impl true
  def mount(_params, session, socket) do
    {:ok,
     assign(socket,
       owner: session["owner"],
       quiz: nil,
       unavailable: false,
       preparing: false,
       error: nil,
       book: "",
       chapter: "",
       step: :book,
       reading: nil,
       reader: nil,
       reader_error: nil
     )}
  end

  @impl true
  def handle_params(params, _uri, socket) do
    case params do
      %{"id" => id} ->
        case Quizzes.fetch(id, socket.assigns.owner) do
          {:ok, quiz} ->
            {:noreply,
             assign(socket,
               quiz: quiz,
               unavailable: false,
               error: nil,
               book: "",
               chapter: "",
               step: :book
             )}

          {:error, _} ->
            {:noreply, assign(socket, quiz: nil, unavailable: true)}
        end

      _ ->
        {:noreply, assign(socket, quiz: nil, unavailable: false)}
    end
  end

  @impl true
  def handle_event("start", _, %{assigns: %{preparing: true}} = socket), do: {:noreply, socket}

  def handle_event("start", _, socket) do
    owner = socket.assigns.owner

    {:noreply,
     socket
     |> assign(preparing: true, error: nil)
     |> start_async(:prepare, fn -> Quizzes.create(owner) end)}
  end

  def handle_event("choose-book", %{"book" => book}, socket) do
    if editable?(socket) and book in books(socket.assigns.quiz.snapshot) do
      {:noreply, assign(socket, book: book, chapter: "", step: :chapter, error: nil)}
    else
      {:noreply, socket}
    end
  end

  def handle_event("choose-chapter", %{"chapter" => chapter}, socket) do
    if editable?(socket) and
         chapter in Enum.map(
           chapters(socket.assigns.quiz.snapshot, socket.assigns.book),
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
        %{assigns: %{quiz: %{id: id}}} = socket
      )
      when is_binary(index) do
    with {index, ""} <- Integer.parse(index) do
      update(
        socket,
        Quizzes.submit(id, socket.assigns.owner, index, book, chapter)
      )
    else
      _ -> {:noreply, assign(socket, error: "Choose a valid book and chapter.")}
    end
  end

  def handle_event("advance", %{"round" => index}, %{assigns: %{quiz: %{id: id}}} = socket)
      when is_binary(index) do
    with {index, ""} <- Integer.parse(index) do
      update(socket, Quizzes.advance(id, socket.assigns.owner, index))
    else
      _ -> {:noreply, socket}
    end
  end

  def handle_event(
        "read-chapter",
        %{"round" => index},
        %{assigns: %{quiz: %{snapshot: snapshot}}} = socket
      )
      when is_binary(index) do
    with {index, ""} when index >= 0 <- Integer.parse(index),
         %{"guess" => guess} = round when not is_nil(guess) <- Enum.at(snapshot["rounds"], index) do
      {:noreply, load_chapter(socket, round)}
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
  def handle_async(:prepare, {:ok, {:ok, quiz}}, socket) do
    {:noreply, socket |> assign(preparing: false) |> push_navigate(to: "/quiz/#{quiz.id}")}
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
           "The corpus has changed since this quiz. This chapter is unavailable for this session."
       )}

  def handle_async(:chapter, _result, socket),
    do: {:noreply, assign(socket, reader_error: "Couldn’t load this chapter. Try again.")}

  defp load_chapter(socket, round) do
    release = socket.assigns.quiz.snapshot["release"]

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

  defp update(socket, {:ok, quiz}),
    do:
      {:noreply,
       assign(close_reader(socket), quiz: quiz, book: "", chapter: "", step: :book, error: nil)}

  defp update(socket, {:error, :invalid_guess}),
    do: {:noreply, assign(socket, error: "Choose a valid book and chapter.")}

  defp update(socket, {:error, _}) do
    case Quizzes.fetch(socket.assigns.quiz.id, socket.assigns.owner) do
      {:ok, quiz} ->
        {:noreply,
         assign(socket,
           quiz: quiz,
           error: "This round changed in another tab. Your saved progress is shown below."
         )}

      _ ->
        {:noreply, assign(socket, quiz: nil, unavailable: true)}
    end
  end

  defp reference(round), do: "#{round["book"]} #{round["chapter"]}:#{round["verse"]}"
  defp books(snapshot), do: snapshot["chapters"] |> Enum.map(& &1["book"]) |> Enum.uniq()

  defp chapters(snapshot, book),
    do: snapshot["chapters"] |> Enum.filter(&(&1["book"] == book)) |> Enum.map(& &1["chapter"])

  defp editable?(%{assigns: %{quiz: %{snapshot: snapshot}}}) do
    !snapshot["finished"] and is_nil(Enum.at(snapshot["rounds"], snapshot["current"])["guess"])
  end

  defp editable?(_), do: false

  defp distance_label(0), do: "Exact chapter"
  defp distance_label(1), do: "Off by 1 chapter"
  defp distance_label(n), do: "Off by #{n} chapters"

  @impl true
  def render(assigns) do
    snapshot = if assigns.quiz, do: assigns.quiz.snapshot

    assigns =
      assign(assigns,
        snapshot: snapshot,
        round: if(snapshot, do: Enum.at(snapshot["rounds"], snapshot["current"]))
      )

    quiz(assigns)
  end

  embed_templates "quiz.html"
end
