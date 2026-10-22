defmodule Scripguessr.Quizzes do
  import Ecto.Query
  alias Scripguessr.{Repo, Corpus.Cache}
  alias Scripguessr.Quizzes.{Quiz, Rules}

  def create(owner, user_id \\ nil) when is_binary(owner) do
    with {:ok, snapshot} <- Cache.prepare() do
      %Quiz{owner_hash: owner_hash(owner), user_id: user_id, snapshot: snapshot} |> Repo.insert()
    end
  end

  def fetch(id, owner, user_id \\ nil) do
    with {:ok, uuid} <- Ecto.UUID.cast(id),
         %Quiz{} = quiz <- Repo.one(owned(uuid, owner, user_id)) do
      {:ok, quiz}
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

          quiz ->
            case update.(quiz.snapshot) do
              {:ok, snapshot} ->
                quiz |> Ecto.Changeset.change(snapshot: snapshot) |> Repo.update!()

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
    from(q in Quiz,
      where: q.user_id == ^user_id,
      order_by: [desc: q.inserted_at, desc: q.id],
      limit: 21,
      offset: ^((page - 1) * 20)
    )
    |> Repo.all()
  end

  def unclaimed_count(owner),
    do: Repo.aggregate(unclaimed(owner), :count)

  def claim(owner, user_id) when is_binary(user_id) do
    # A single conditional UPDATE prevents two accounts claiming the same quiz.
    {count, _} = Repo.update_all(unclaimed(owner), set: [user_id: user_id])
    count
  end

  defp unclaimed(owner),
    do: from(q in Quiz, where: is_nil(q.user_id) and q.owner_hash == ^owner_hash(owner))

  defp owned(id, owner, nil),
    do: from(q in unclaimed(owner), where: q.id == ^id)

  defp owned(id, owner, user_id),
    do:
      from(q in Quiz,
        where:
          q.id == ^id and
            (q.user_id == ^user_id or (is_nil(q.user_id) and q.owner_hash == ^owner_hash(owner)))
      )

  defp owner_hash(owner), do: :crypto.hash(:sha256, owner)
end
