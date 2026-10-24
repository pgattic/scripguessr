defmodule Scripguessr.Leaderboards do
  @moduledoc "Public player rankings derived from earned game rewards."
  import Ecto.Query
  alias Scripguessr.Accounts.User
  alias Scripguessr.Progression.{Reward, Rules}
  alias Scripguessr.Repo

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
