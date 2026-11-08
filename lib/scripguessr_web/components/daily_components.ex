defmodule ScripguessrWeb.DailyComponents do
  use Phoenix.Component

  attr :id, :string, required: true
  attr :day, Date, required: true
  attr :label, :string, default: "Resets"
  attr :countdown, :boolean, default: true

  def daily_deadline(assigns) do
    deadline = assigns.day |> Date.add(1) |> DateTime.new!(~T[00:00:00], "Etc/UTC")
    assigns = assign(assigns, deadline: deadline, now: DateTime.utc_now())

    ~H"""
    <span
      id={@id}
      class="daily-deadline"
      data-daily-deadline
      data-deadline={DateTime.to_iso8601(@deadline)}
      data-server-now={DateTime.to_iso8601(@now)}
      data-label={@label}
      title="Daily challenges reset worldwide at 00:00 UTC."
    >
      <time datetime={DateTime.to_iso8601(@deadline)} data-deadline-time>
        {@label} {Calendar.strftime(@deadline, "%b %-d at %H:%M UTC")}
      </time><span :if={@countdown} data-deadline-countdown></span>
    </span>
    """
  end
end
