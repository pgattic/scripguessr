defmodule ScripguessrWeb.StreakComponents do
  use Phoenix.Component
  import ScripguessrWeb.Icons
  import ScripguessrWeb.DailyComponents

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
      <p class="muted">Complete all five rounds before each daily reset to keep your streak.</p>
      <p class="muted">
        <.daily_deadline id="streak-deadline" day={Date.utc_today()} label="Next reset:" />
      </p>
    </section>
    """
  end

  defp days(count), do: "#{count} #{if count == 1, do: "day", else: "days"}"
end
