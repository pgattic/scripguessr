defmodule ScripguessrWeb.ProgressionComponents do
  use Phoenix.Component

  attr :id, :string, required: true
  attr :progress, :map, required: true

  def level_progress(assigns) do
    ~H"""
    <div id={@id} class="level-progress">
      <div class="xp-heading">
        <strong>Level {@progress.level}</strong><span class="muted">{@progress.xp} XP total</span>
      </div>
      <div
        class="xp-track"
        role="progressbar"
        aria-label={"Progress toward level #{@progress.level + 1}"}
        aria-valuemin="0"
        aria-valuemax={@progress.needed}
        aria-valuenow={@progress.earned}
        aria-valuetext={"#{@progress.earned} of #{@progress.needed} XP toward level #{@progress.level + 1}"}
      >
        <span style={"width: #{@progress.percent}%"}></span>
      </div>
      <p class="muted xp-next">{@progress.remaining} XP to level {@progress.level + 1}</p>
    </div>
    """
  end
end
