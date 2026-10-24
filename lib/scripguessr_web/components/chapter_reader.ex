defmodule ScripguessrWeb.ChapterReader do
  use Phoenix.Component
  alias Phoenix.LiveView.JS
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
              class="text-button"
              phx-click={JS.dispatch("chapter:first-match", to: "#" <> @id)}
            >
              {if @target, do: "Verse #{@target}", else: "First match"} ↓
            </button>
            <button
              id={@id <> "-close"}
              data-reader-close
              class="crumb"
              phx-click="close-chapter"
              aria-label="Close chapter"
            >×</button>
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
          <article :if={@verses} class="atlas-reading" aria-label="Chapter text">
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
                (verse["verse"] == @target || Map.has_key?(@marks, verse["verse"])) && "matched"
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
              <span :if={map_size(@marks) > 0} class="atlas-verse-bands" aria-hidden="true">
                <b
                  :for={mark <- Map.get(@marks, verse["verse"], [])}
                  style={"--layer-color: #{mark.color}"}
                ></b>
              </span>
            </p>
          </article>
        </div>
      </.focus_wrap>
    </div>
    """
  end
end
