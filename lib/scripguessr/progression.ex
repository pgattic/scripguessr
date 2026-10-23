defmodule Scripguessr.Progression do
  @moduledoc "Permanent XP awards, with one database-enforced reward per completed game."
  import Ecto.Query
  alias Scripguessr.Repo
  alias Scripguessr.Games.Game
  alias Scripguessr.Progression.{Reward, Rules}

  @doc false
  def award(%Game{user_id: user_id} = game) when not is_nil(user_id) do
    if reward = Rules.reward(game.snapshot) do
      Repo.insert!(
        %Reward{
          game_id: game.id,
          user_id: user_id,
          completion_xp: reward.completion_xp,
          accuracy_xp: reward.accuracy_xp
        },
        on_conflict: :nothing,
        conflict_target: :game_id
      )
    end

    :ok
  end

  def award(%Game{}), do: :ok

  def for_game(game_id), do: Repo.get(Reward, game_id)

  def for_user(user_id) do
    {:ok, xp} =
      Repo.transaction(fn ->
        # Credit older completed games lazily. Reward amounts are stored once,
        # so future tuning cannot silently rewrite already-earned XP.
        missing =
          from g in Game,
            left_join: r in Reward,
            on: r.game_id == g.id,
            where: g.user_id == ^user_id and is_nil(r.game_id),
            where: fragment("?->>'finished' = 'true'", g.snapshot),
            order_by: g.id,
            select: g

        missing |> Repo.all() |> Enum.each(&award/1)

        Repo.one(
          from r in Reward,
            where: r.user_id == ^user_id,
            select: coalesce(sum(r.completion_xp + r.accuracy_xp), 0)
        )
      end)

    Rules.progress(xp)
  end
end
