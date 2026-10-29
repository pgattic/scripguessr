defmodule Scripguessr.Streaks do
  @moduledoc "Daily completion streaks derived from saved challenge dates."
  import Ecto.Query
  alias Scripguessr.{Repo, Games.Game, Challenges.Challenge}

  def for_user(user_id, today \\ Date.utc_today())
  def for_user(nil, _today), do: %{current: 0, best: 0, completed_today: false}

  def for_user(user_id, today) do
    days =
      Repo.all(
        from g in Game,
          join: c in Challenge,
          on: c.id == g.challenge_id,
          where: g.user_id == ^user_id and not is_nil(g.score) and c.day <= ^today,
          distinct: true,
          order_by: [asc: c.day],
          select: c.day
      )

    {last, run, best} =
      Enum.reduce(days, {nil, 0, 0}, fn day, {previous, run, best} ->
        run = if previous && Date.diff(day, previous) == 1, do: run + 1, else: 1
        {day, run, max(best, run)}
      end)

    %{
      current: if(last && Date.diff(today, last) in 0..1, do: run, else: 0),
      best: best,
      completed_today: last == today
    }
  end
end
