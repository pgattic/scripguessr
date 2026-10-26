defmodule ScripguessrWeb.GameComponents do
  use Phoenix.Component

  attr :text, :string, required: true
  attr :reference, :string, default: nil

  def verse_panel(assigns) do
    ~H"""
    <article class="panel verse-panel">
      <div class="verse-label">
        <span>Book of Mormon</span><span :if={@reference}>{@reference}</span>
      </div>
      <blockquote id="verse-text" class="verse" tabindex="-1">{@text}</blockquote>
    </article>
    """
  end

  attr :chapters, :list, required: true
  attr :book, :string, required: true
  attr :chapter, :string, required: true
  attr :step, :atom, required: true
  attr :round, :integer, required: true
  attr :error, :string, default: nil
  attr :revealed, :boolean, default: false
  attr :title, :string, default: "Result"
  slot :inner_block

  def picker(assigns) do
    assigns =
      assign(assigns,
        books: Enum.uniq(Enum.map(assigns.chapters, & &1["book"])),
        numbers: chapter_numbers(assigns.chapters, assigns.book)
      )

    ~H"""
    <aside
      id="picker"
      class="panel picker"
      phx-hook="PickerFocus"
      data-step={if @revealed, do: "result", else: @step}
    >
      <div class="picker-header">
        <h1 id="picker-heading" tabindex="-1">{if @revealed, do: @title, else: "Guess location"}</h1>
        <span :if={!@revealed} class="muted">{case @step do
          :book -> "Book"
          :chapter -> "Chapter"
          :ready -> "Confirm"
        end}</span>
      </div>
      <p :if={@error} class="error" role="alert">{@error}</p>
      <nav :if={!@revealed && @book != ""} class="breadcrumbs" aria-label="Your guess">
        <button
          id="edit-book"
          class={if @step == :chapter || length(@numbers) == 1, do: "crumb current", else: "crumb"}
          phx-click="edit-book"
        >{@book}</button>
        <span :if={@chapter != "" && length(@numbers) > 1} aria-hidden="true">/</span>
        <button
          :if={@chapter != "" && length(@numbers) > 1}
          id="edit-chapter"
          class={if @step == :ready, do: "crumb current", else: "crumb"}
          phx-click="edit-chapter"
        >Chapter {@chapter}</button>
        <button
          id="clear-guess"
          class="crumb clear-guess"
          phx-click="clear-guess"
          aria-label="Clear guess"
          title="Clear guess"
        >×</button>
      </nav>
      <div :if={!@revealed && @step == :book} id="book-grid" class="grid book-grid">
        <button
          :for={book <- @books}
          class={if book == @book, do: "choice active", else: "choice"}
          phx-click="choose-book"
          phx-value-book={book}
          aria-pressed={book == @book}
        >{book}</button>
      </div>
      <div :if={!@revealed && @step == :chapter} id="chapter-grid" class="grid chapter-grid">
        <button
          :for={number <- @numbers}
          class={if @chapter == to_string(number), do: "choice active", else: "choice"}
          phx-click="choose-chapter"
          phx-value-chapter={number}
          aria-label={"Chapter #{number}"}
          aria-pressed={@chapter == to_string(number)}
        >{number}</button>
      </div>
      <form :if={!@revealed && @step == :ready} id="guess-form" phx-submit="guess">
        <input type="hidden" name="guess[round]" value={@round} />
        <input type="hidden" name="guess[book]" value={@book} />
        <input type="hidden" name="guess[chapter]" value={@chapter} />
        <div class="ready">
          <span class="muted">Your guess</span><strong>{@book} {@chapter}</strong>
        </div>
        <button class="button full-width" phx-disable-with="Submitting…">Submit guess</button>
      </form>
      {render_slot(@inner_block)}
    </aside>
    """
  end

  attr :id, :string, default: "answer-reveal"
  attr :answer, :string, required: true
  attr :round, :integer, required: true

  attr :wide, :boolean, default: false

  slot :player, required: true do
    attr :rank, :integer
    attr :name, :string, required: true
    attr :guess, :string, required: true
    attr :points, :integer, required: true
    attr :distance, :integer
    attr :guest, :boolean
    attr :self, :boolean
  end

  slot :inner_block

  def round_feedback(assigns) do
    ~H"""
    <div id={@id} class={["result", @wide && "round-results-wide"]} role="status">
      <div class="round-answer">
        <span class="muted">The answer</span><strong>{@answer}</strong>
      </div>
      <div :if={@wide} class="round-results-columns muted" aria-hidden="true">
        <span>Player</span><span>Guess</span><span>Distance</span><span>Points</span>
      </div>
      <ul class="round-players" aria-label="Round results">
        <li :for={player <- @player} class="round-player" data-self={to_string(player[:self] == true)}>
          <div class="round-player-heading">
            <span
              :if={@wide && player[:rank]}
              class="round-player-rank"
              aria-label={"Place #{player.rank}"}
            >{player.rank}</span>
            <strong class="round-player-name">{player.name}</strong>
            <span :if={player[:self] && player.name != "You"} class="pill">You</span>
            <span :if={player[:guest]} class="pill">Guest</span>
          </div>
          <span class="score">{player.points}<span> / 1,000</span></span>
          <strong class="round-player-guess">{player.guess}</strong>
          <p class="distance">{distance_label(player[:distance])}</p>
          <div
            class="score-meter"
            aria-hidden="true"
            data-meter={player.points / 10}
            data-meter-stage={@round}
          >
            <span data-meter-fill style={"width: #{player.points / 10}%"}></span>
          </div>
        </li>
      </ul>
      {render_slot(@inner_block)}
    </div>
    """
  end

  def chapter_numbers(chapters, book),
    do: chapters |> Enum.filter(&(&1["book"] == book)) |> Enum.map(& &1["chapter"])

  def distance_label(nil), do: "No answer submitted"
  def distance_label(0), do: "Exact chapter"
  def distance_label(1), do: "Off by 1 chapter"
  def distance_label(n), do: "Off by #{n} chapters"
end
