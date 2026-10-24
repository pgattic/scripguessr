defmodule Scripguessr.Leaderboards do
  @moduledoc "Public player rankings derived from earned game rewards."
  import Ecto.Query
  alias Scripguessr.Accounts.User
  alias Scripguessr.Progression.{Reward, Rules}
  alias Scripguessr.Repo
  alias Scripguessr.Games.Game
  alias Scripguessr.Challenges.Challenge

  @doc "The top 50 completed attempts for a UTC day, with shared score ranks."
  def daily(day), do: daily_query(day) |> limit(50) |> Repo.all()

  def daily_rank(_day, nil), do: nil

  def daily_rank(day, user_id) do
    Repo.one(from player in subquery(daily_query(day)), where: player.user_id == ^user_id)
  end

  defp daily_query(day) do
    from g in Game,
      join: c in Challenge,
      on: c.id == g.challenge_id,
      join: u in User,
      on: u.id == g.user_id,
      where: c.day == ^day and not is_nil(g.score),
      order_by: [desc: g.score, asc: u.username_normalized],
      select: %{
        user_id: u.id,
        username: u.username,
        score: g.score,
        rank: fragment("rank() OVER (ORDER BY ? DESC)", g.score)
      }
  end

  @doc "The top 50 players by lifetime XP. Equal XP shares a competition rank."
  def xp do
    totals =
      from r in Reward,
        group_by: r.user_id,
        select: %{user_id: r.user_id, xp: sum(r.completion_xp + r.accuracy_xp)}

    from(u in User,
      join: total in subquery(totals),
      on: total.user_id == u.id,
      where: total.xp > 0,
      order_by: [desc: total.xp, asc: u.username_normalized],
      limit: 50,
      select: %{
        user_id: u.id,
        username: u.username,
        xp: total.xp,
        rank: fragment("rank() OVER (ORDER BY ? DESC)", total.xp)
      }
    )
    |> Repo.all()
    |> Enum.map(&Map.put(&1, :level, Rules.progress(&1.xp).level))
  end
end
