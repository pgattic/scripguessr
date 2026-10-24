defmodule ScripguessrWeb.MatchPicker do
  use Phoenix.Component

  attr :chapters, :list, required: true
  attr :book, :string, required: true
  attr :chapter, :string, required: true
  attr :step, :atom, required: true
  attr :round, :integer, required: true
  attr :error, :string, default: nil

  def picker(assigns) do
    assigns =
      assign(assigns,
        books: Enum.uniq(Enum.map(assigns.chapters, & &1["book"])),
        numbers: chapter_numbers(assigns.chapters, assigns.book)
      )

    ~H"""
    <aside id="picker" class="panel picker" phx-hook="PickerFocus" data-step={@step}>
      <div class="picker-header">
        <h1 id="picker-heading" tabindex="-1">Guess location</h1><span class="muted">{case @step do
          :book -> "Book"
          :chapter -> "Chapter"
          :ready -> "Confirm"
        end}</span>
      </div>
      <p :if={@error} class="error" role="alert">{@error}</p>
      <nav :if={@book != ""} class="breadcrumbs" aria-label="Your guess">
        <button id="edit-book" class="crumb" phx-click="edit-book">{@book}</button>
        <span :if={@chapter != "" && length(@numbers) > 1} aria-hidden="true">/</span>
        <button
          :if={@chapter != "" && length(@numbers) > 1}
          id="edit-chapter"
          class="crumb"
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
      <div :if={@step == :book} id="book-grid" class="grid book-grid">
        <button :for={book <- @books} class="choice" phx-click="choose-book" phx-value-book={book}>{book}</button>
      </div>
      <div :if={@step == :chapter} id="chapter-grid" class="grid chapter-grid">
        <button
          :for={number <- @numbers}
          class={if @chapter == to_string(number), do: "choice active", else: "choice"}
          phx-click="choose-chapter"
          phx-value-chapter={number}
          aria-label={"Chapter #{number}"}
          aria-pressed={@chapter == to_string(number)}
        >{number}</button>
      </div>
      <form :if={@step == :ready} id="guess-form" phx-submit="guess">
        <input type="hidden" name="guess[round]" value={@round} />
        <input type="hidden" name="guess[book]" value={@book} />
        <input type="hidden" name="guess[chapter]" value={@chapter} />
        <div class="ready">
          <span class="muted">Your guess</span><strong>{@book} {@chapter}</strong>
        </div>
        <button class="button full-width" phx-disable-with="Submitting…">Submit guess</button>
      </form>
    </aside>
    """
  end

  def chapter_numbers(chapters, book),
    do: chapters |> Enum.filter(&(&1["book"] == book)) |> Enum.map(& &1["chapter"])
end
