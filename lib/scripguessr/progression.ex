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
    xp =
      Repo.one(
        from r in Reward,
          where: r.user_id == ^user_id,
          select: coalesce(sum(r.completion_xp + r.accuracy_xp), 0)
      )

    Rules.progress(xp)
  end
end
