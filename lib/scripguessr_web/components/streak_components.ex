defmodule ScripguessrWeb.StreakComponents do
  use Phoenix.Component
  import ScripguessrWeb.Icons

  attr :id, :string, required: true
  attr :count, :integer, required: true

  def streak_badge(assigns) do
    ~H"""
    <span id={@id} class="streak-badge"><.icon name="flame" /><span>{@count}-day streak</span></span>
    """
  end

  attr :streak, :map, required: true

  def streak_stats(assigns) do
    ~H"""
    <section id="account-streaks" class="panel streak-stats" aria-labelledby="streaks-title">
      <h2 id="streaks-title">Daily streaks</h2>
      <dl>
        <div>
          <dt>Current streak</dt><dd id="current-streak">{days(@streak.current)}</dd>
        </div>
        <div>
          <dt>Best streak</dt><dd id="best-streak">{days(@streak.best)}</dd>
        </div>
      </dl>
      <p class="muted">Complete a daily challenge each day. Days reset at midnight UTC.</p>
    </section>
    """
  end

  defp days(count), do: "#{count} #{if count == 1, do: "day", else: "days"}"
end
