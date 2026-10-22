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
       chapter: ""
     )}
  end

  @impl true
  def handle_params(params, _uri, socket) do
    case params do
      %{"id" => id} ->
        case Quizzes.fetch(id, socket.assigns.owner) do
          {:ok, quiz} ->
            {:noreply,
             assign(socket, quiz: quiz, unavailable: false, error: nil, book: "", chapter: "")}

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

  def handle_event("select", %{"guess" => params}, socket) do
    book = params["book"] || ""
    chapter = if book == socket.assigns.book, do: params["chapter"] || "", else: ""
    {:noreply, assign(socket, book: book, chapter: chapter, error: nil)}
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

  def handle_event(_event, _params, socket), do: {:noreply, socket}

  @impl true
  def handle_async(:prepare, {:ok, {:ok, quiz}}, socket) do
    {:noreply, socket |> assign(preparing: false) |> push_navigate(to: "/quiz/#{quiz.id}")}
  end

  def handle_async(:prepare, _result, socket) do
    {:noreply,
     assign(socket,
       preparing: false,
       error:
         "We couldn’t prepare your verses. The scripture service may be busy. Please try again shortly."
     )}
  end

  defp update(socket, {:ok, quiz}),
    do: {:noreply, assign(socket, quiz: quiz, book: "", chapter: "", error: nil)}

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

  @impl true
  def render(assigns) do
    assigns = assign(assigns, :snapshot, if(assigns.quiz, do: assigns.quiz.snapshot))

    ~H"""
    <section :if={@unavailable} class="welcome narrow">
      <p class="eyebrow">A PRIVATE PLACE TO PRACTICE</p>
      <h1>This quiz isn’t available.</h1>
      <p>Return using the browser where you started it, or begin a fresh set of verses.</p>
      <a class="button" href="/">Back to the beginning <span aria-hidden="true">↗</span></a>
    </section>

    <section :if={!@quiz && !@unavailable} class="welcome">
      <div class="intro-copy">
        <p class="eyebrow"><span class="small-line"></span> BOOK OF MORMON · FIVE VERSES</p>
        <h1>Familiar words.<br />A fresh perspective.</h1>
        <p class="lede">
          Some verses you know by heart. Others are waiting to be rediscovered. Read a passage, find its chapter, and see how close you come.
        </p>
        <button
          id="start-quiz"
          class="button"
          phx-click="start"
          disabled={@preparing}
          phx-disable-with="Gathering your verses…"
        >
          {if @preparing, do: "Gathering your verses…", else: "Find your first verse"}<span aria-hidden="true">↗</span>
        </button>
        <p :if={@preparing} role="status" class="subtle">
          Preparing five verses from the corpus. This may take a moment.
        </p>
        <p :if={@error} role="alert" class="error">{@error}</p>
        <div class="intro-details">
          <span>5 rounds</span><span>No timer</span><span>Just you & the text</span>
        </div>
      </div>
      <aside class="intro-art" aria-hidden="true">
        <div class="orbit orbit-one"></div><div class="orbit orbit-two"></div>
        <div class="art-star">✳</div><div class="art-center">A moment<br /><em>in the word.</em></div>
        <span class="art-coordinate">READ / RECALL / REDISCOVER</span>
      </aside>
      <div class="how-it-works">
        <div>
          <span class="step-number">01</span><h2>Read the verse</h2><p>
            Take your time. Notice the words, the voice, the story.
          </p>
        </div>
        <div>
          <span class="step-number">02</span><h2>Find its home</h2><p>
            Choose a book and chapter. A close guess counts, too.
          </p>
        </div>
        <div>
          <span class="step-number">03</span><h2>Make a connection</h2><p>
            Reveal the reference. Leave knowing a little more.
          </p>
        </div>
      </div>
    </section>

    <section :if={@quiz && !@snapshot["finished"]} class="quiz-page">
      <div class="quiz-heading">
        <div>
          <p class="eyebrow">BOOK OF MORMON</p><h1>Find this verse.</h1>
        </div><p class="score-label"><strong>{Rules.total(@snapshot)}</strong> points collected</p>
      </div>
      <div class="progress-header">
        <span>ROUND {@snapshot["current"] + 1} <span class="muted">OF 5</span></span><span>There’s no hurry.</span>
      </div>
      <div class="progress-track" aria-label={"Round #{@snapshot["current"] + 1} of 5"}>
        <span :for={n <- 0..4} class={if n <= @snapshot["current"], do: "filled"}></span>
      </div>
      <% round = Enum.at(@snapshot["rounds"], @snapshot["current"]) %>
      <article class="verse-card">
        <span class="quote-mark" aria-hidden="true">“</span><p id="verse-text">{round["text"]}</p><span class="verse-caption">{if round[
                                                                                                                                    "guess"
                                                                                                                                  ],
                                                                                                                                  do:
                                                                                                                                    reference(
                                                                                                                                      round
                                                                                                                                    ),
                                                                                                                                  else:
                                                                                                                                    "Where do these words belong?"}</span>
      </article>
      <p :if={@error} role="alert" class="error">{@error}</p>
      <div :if={!round["guess"]} class="guess-panel">
        <div>
          <p class="eyebrow">YOUR BEARINGS</p><h2>Book & chapter</h2><p class="subtle">
            An exact chapter earns 1,000 points.<br />Each chapter away costs 22.
          </p>
        </div>
        <form id="guess-form" phx-change="select" phx-submit="guess">
          <input type="hidden" name="guess[round]" value={@snapshot["current"]} />
          <div class="select-row">
            <div>
              <label for="book">Book</label><select id="book" name="guess[book]" required><option value="">
                Choose a book
              </option><option :for={book <- books(@snapshot)} value={book} selected={book == @book}>
                {book}
              </option></select>
            </div>
            <div>
              <label for="chapter">Chapter</label><select
                id="chapter"
                name="guess[chapter]"
                required
                disabled={@book == ""}
              ><option value="">Choose</option><option
                :for={chapter <- chapters(@snapshot, @book)}
                value={chapter}
                selected={to_string(chapter) == @chapter}
              >
                {chapter}
              </option></select>
            </div>
          </div>
          <button
            class="button"
            disabled={@book == "" || @chapter == ""}
            phx-disable-with="Checking…"
          >Reveal the reference <span aria-hidden="true">→</span></button>
        </form>
      </div>
      <div :if={round["guess"]} id="answer-reveal" class="answer-panel" role="status">
        <div>
          <p class="eyebrow">
            {if round["guess"]["distance"] == 0, do: "RIGHT AT HOME", else: "A NEW CONNECTION"}
          </p><h2>{reference(round)}</h2><p>
            You guessed {round["guess"]["book"]} {round["guess"]["chapter"]} · {round["guess"][
              "distance"
            ]} chapters away
          </p>
        </div>
        <div class="round-points">+{round["guess"]["points"]}<span>POINTS</span></div>
        <button
          id="next-round"
          class="button"
          phx-click="advance"
          phx-value-round={@snapshot["current"]}
          phx-disable-with="Saving…"
        >{if @snapshot["current"] == 4, do: "See your reflections", else: "Next verse"}<span aria-hidden="true">→</span></button>
      </div>
      <p class="saved-note">Your place is saved. Come back whenever you like.</p>
    </section>

    <section :if={@quiz && @snapshot["finished"]} class="summary-page">
      <p class="eyebrow">FIVE VERSES. A FEW NEW CONNECTIONS.</p><h1>A little better acquainted.</h1>
      <p class="lede">Every passage has a place. Thanks for spending a moment finding it.</p>
      <div class="final-score">
        <strong>{Rules.total(@snapshot)}</strong><span>OUT OF 5,000 POINTS</span>
      </div>
      <div class="round-list">
        <article :for={{round, index} <- Enum.with_index(@snapshot["rounds"])}>
          <span class="step-number">0{index + 1}</span><div>
            <h2>{reference(round)}</h2><p>{round["text"]}</p><span class="subtle">Your guess: {round[
              "guess"
            ]["book"]} {round["guess"]["chapter"]} · {round["guess"]["distance"]} chapters away</span>
          </div><strong>{round["guess"]["points"]}<small>pts</small></strong>
        </article>
      </div>
      <a href="/" class="button">Another five verses <span aria-hidden="true">↗</span></a>
    </section>
    """
  end
end
