defmodule ScripguessrWeb.ChapterReader do
  use Phoenix.Component
  alias Phoenix.LiveView.JS
  import ScripguessrWeb.Icons
  import ScripguessrWeb.SavedVerseComponents, only: [save_button: 1]

  attr :id, :string, required: true
  attr :chapter, :any, required: true
  attr :verses, :any, required: true
  attr :error, :string, default: nil
  attr :save_error, :string, default: nil
  attr :user, :any, required: true
  attr :saved, :any, required: true
  attr :target, :integer, default: nil
  attr :marks, :map, default: %{}
  attr :retry_event, :string, default: "retry-chapter"
  slot :before_text
  slot :error_actions

  def chapter_reader(assigns) do
    lanes =
      assigns.marks
      |> Enum.flat_map(fn {number, marks} -> if number > 0, do: marks, else: [] end)
      |> Enum.uniq_by(& &1.key)
      |> Enum.sort_by(& &1.key)

    assigns = assign(assigns, :lanes, lanes)

    ~H"""
    <div
      :if={@chapter}
      id={@id}
      class="atlas-reader-backdrop"
      phx-hook="ChapterReader"
      data-chapter={inspect(@chapter)}
      data-ready={to_string(!is_nil(@verses))}
      data-auto-scroll={to_string(!is_nil(@target))}
      phx-window-keydown="close-chapter"
      phx-key="Escape"
    >
      <.focus_wrap
        id={@id <> "-dialog"}
        class="atlas-reader-dialog panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby={@id <> "-title"}
        phx-click-away="close-chapter"
      >
        <header class="atlas-reader-header">
          <h2 id={@id <> "-title"}>{elem(@chapter, 0)} {elem(@chapter, 1)}</h2>
          <div class="reader-actions">
            <button
              :if={@verses && (@target || map_size(@marks) > 0)}
              type="button"
              class="text-button icon-label"
              phx-click={JS.dispatch("chapter:first-match", to: "#" <> @id)}
            >
              <.icon name="down" />
              <span>{if @target, do: "Verse #{@target}", else: "First match"}</span>
            </button>
            <.icon_button
              id={@id <> "-close"}
              data-reader-close
              name="close"
              label="Close reader"
              phx-click="close-chapter"
            />
          </div>
        </header>
        <div class="atlas-reader-scroll">
          <p :if={@save_error} role="alert" class="error">{@save_error}</p>
          {render_slot(@before_text)}
          <p :if={is_nil(@verses) && !@error} class="atlas-reader-note muted" role="status">
            Loading chapter…
          </p>
          <div :if={@error} class="error" role="alert">
            {@error} <button class="text-button" phx-click={@retry_event}>Retry</button>
            {render_slot(@error_actions)}
          </div>
          <article
            :if={@verses}
            class="atlas-reading"
            aria-label="Chapter text"
            style={"--reader-band-width: #{max(4, length(@lanes) * 4 - 1)}px"}
          >
            <details
              :for={verse <- @verses}
              :if={verse["verse"] == 0}
              class={["atlas-reading-heading", Map.has_key?(@marks, 0) && "matched"]}
              open={Map.has_key?(@marks, 0)}
            >
              <summary>Chapter heading</summary><p>{verse["text"]}</p>
            </details>
            <p
              :for={verse <- @verses}
              :if={verse["verse"] > 0}
              class={[
                "atlas-reading-verse",
                highlighted?(@marks, @target, verse["verse"]) && "matched",
                highlighted?(@marks, @target, verse["verse"]) &&
                  highlighted?(@marks, @target, verse["verse"] - 1) && "highlight-joins-before",
                highlighted?(@marks, @target, verse["verse"]) &&
                  highlighted?(@marks, @target, verse["verse"] + 1) && "highlight-joins-after"
              ]}
              data-verse={verse["verse"]}
              aria-current={if verse["verse"] == @target, do: "true"}
              title={Enum.map_join(Map.get(@marks, verse["verse"], []), ", ", & &1.label)}
            >
              <sup>{verse["verse"]}</sup><span>{verse["text"]}</span>
              <.save_button
                verse={
                  Map.merge(verse, %{"book" => elem(@chapter, 0), "chapter" => elem(@chapter, 1)})
                }
                saved={@saved}
                user={@user}
              />
              <span :if={@lanes != []} class="atlas-verse-bands" aria-hidden="true">
                <b
                  :for={mark <- @lanes}
                  class={[
                    marked?(@marks, verse["verse"], mark.key) && "active",
                    marked?(@marks, verse["verse"], mark.key) &&
                      marked?(@marks, verse["verse"] - 1, mark.key) && "band-joins-before",
                    marked?(@marks, verse["verse"], mark.key) &&
                      marked?(@marks, verse["verse"] + 1, mark.key) && "band-joins-after"
                  ]}
                  style={"--layer-color: #{mark.color}"}
                  data-layer={mark.key}
                ></b>
              </span>
            </p>
          </article>
        </div>
      </.focus_wrap>
    </div>
    """
  end

  defp highlighted?(marks, target, number),
    do: number > 0 && (number == target || Map.has_key?(marks, number))

  defp marked?(marks, number, key),
    do: number > 0 && Enum.any?(Map.get(marks, number, []), &(&1.key == key))
end
