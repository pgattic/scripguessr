defmodule Scripguessr.Games do
  import Ecto.Query
  alias Scripguessr.{Repo, Corpus.Cache, Progression, Challenges}
  alias Scripguessr.Challenges.Challenge
  alias Scripguessr.Games.{Game, Rules}

  def create(owner, user_id \\ nil) when is_binary(owner) do
    with {:ok, snapshot} <- Cache.prepare() do
      %Game{owner_hash: owner_hash(owner), user_id: user_id, snapshot: snapshot} |> Repo.insert()
    end
  end

  def start_daily(_owner, nil), do: {:error, :account_required}

  def start_daily(owner, user_id) when is_binary(owner) and is_binary(user_id) do
    with {:ok, challenge} <- Challenges.daily(),
         true <- challenge.day == Date.utc_today() do
      Repo.insert!(
        %Game{
          owner_hash: owner_hash(owner),
          user_id: user_id,
          challenge_id: challenge.id,
          snapshot: challenge.snapshot
        },
        on_conflict: :nothing,
        conflict_target: [:challenge_id, :user_id]
      )

      {:ok,
       Repo.get_by!(Game, challenge_id: challenge.id, user_id: user_id)
       |> Repo.preload(:challenge)}
    else
      false -> {:error, :daily_closed}
      error -> error
    end
  end

  def daily_for_user(_day, nil), do: nil

  def daily_for_user(day, user_id) do
    Repo.one(
      from g in Game,
        join: c in Challenge,
        on: c.id == g.challenge_id,
        where: c.day == ^day and g.user_id == ^user_id,
        preload: [challenge: c]
    )
  end

  def daily_closed?(%Game{challenge: %Challenge{day: day}}, today), do: day != today
  def daily_closed?(%Game{challenge_id: nil}, _today), do: false

  def fetch(id, owner, user_id \\ nil) do
    with {:ok, uuid} <- Ecto.UUID.cast(id),
         %Game{} = game <- Repo.one(owned(uuid, owner, user_id)) do
      {:ok, Repo.preload(game, :challenge)}
    else
      _ -> {:error, :not_found}
    end
  end

  def submit(id, owner, index, book, chapter, user_id \\ nil),
    do: mutate(id, owner, user_id, &Rules.submit(&1, index, book, chapter))

  def advance(id, owner, index, user_id \\ nil),
    do: mutate(id, owner, user_id, &Rules.advance(&1, index))

  defp mutate(id, owner, user_id, update) do
    with {:ok, uuid} <- Ecto.UUID.cast(id) do
      Repo.transaction(fn ->
        case Repo.one(from q in owned(uuid, owner, user_id), lock: "FOR UPDATE") do
          nil ->
            Repo.rollback(:not_found)

          game ->
            game = Repo.preload(game, :challenge)
            if daily_closed?(game, Date.utc_today()), do: Repo.rollback(:daily_closed)

            case update.(game.snapshot) do
              {:ok, snapshot} ->
                score = if snapshot["finished"], do: Rules.total(snapshot)

                game =
                  game
                  |> Ecto.Changeset.change(snapshot: snapshot, score: score)
                  |> Repo.update!()

                Progression.award(game)
                game

              {:error, reason} ->
                Repo.rollback(reason)
            end
        end
      end)
    else
      _ -> {:error, :not_found}
    end
  end

  def history(user_id, page \\ 1) do
    from(q in Game,
      where: q.user_id == ^user_id,
      order_by: [desc: q.inserted_at, desc: q.id],
      limit: 21,
      offset: ^((page - 1) * 20)
    )
    |> Repo.all()
    |> Repo.preload(:challenge)
  end

  def latest_unfinished(owner, user_id \\ nil) do
    accessible =
      if user_id do
        from g in Game,
          where:
            g.user_id == ^user_id or (is_nil(g.user_id) and g.owner_hash == ^owner_hash(owner))
      else
        unclaimed(owner)
      end

    Repo.one(
      from g in accessible,
        where: fragment("?->>'finished' = 'false'", g.snapshot) and is_nil(g.challenge_id),
        order_by: [desc: g.updated_at, desc: g.id],
        limit: 1,
        select: %{id: g.id, current: fragment("(?->>'current')::integer", g.snapshot)}
    )
  end

  def unclaimed_count(owner),
    do: Repo.aggregate(unclaimed(owner), :count)

  def claim(owner, user_id) when is_binary(user_id) do
    {:ok, count} =
      Repo.transaction(fn ->
        # Claim and credit completed games in the same transaction. Concurrent claims
        # recheck the null owner, and the reward primary key prevents duplicate XP.
        {count, games} =
          Repo.update_all(from(g in unclaimed(owner), select: g), set: [user_id: user_id])

        Enum.each(games, &Progression.award/1)
        count
      end)

    count
  end

  defp unclaimed(owner),
    do: from(q in Game, where: is_nil(q.user_id) and q.owner_hash == ^owner_hash(owner))

  defp owned(id, owner, nil),
    do: from(q in unclaimed(owner), where: q.id == ^id)

  defp owned(id, owner, user_id),
    do:
      from(q in Game,
        where:
          q.id == ^id and
            (q.user_id == ^user_id or (is_nil(q.user_id) and q.owner_hash == ^owner_hash(owner)))
      )

  defp owner_hash(owner), do: :crypto.hash(:sha256, owner)
end
