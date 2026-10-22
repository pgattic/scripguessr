defmodule Scripguessr.Quizzes do
  import Ecto.Query
  alias Scripguessr.{Repo, Corpus.Cache}
  alias Scripguessr.Quizzes.{Quiz, Rules}

  def create(owner) when is_binary(owner) do
    with {:ok, snapshot} <- Cache.prepare() do
      %Quiz{owner_hash: owner_hash(owner), snapshot: snapshot} |> Repo.insert()
    end
  end

  def fetch(id, owner) do
    with {:ok, uuid} <- Ecto.UUID.cast(id),
         %Quiz{} = quiz <- Repo.one(owned(uuid, owner)) do
      {:ok, quiz}
    else
      _ -> {:error, :not_found}
    end
  end

  def submit(id, owner, index, book, chapter),
    do: mutate(id, owner, &Rules.submit(&1, index, book, chapter))

  def advance(id, owner, index), do: mutate(id, owner, &Rules.advance(&1, index))

  defp mutate(id, owner, update) do
    with {:ok, uuid} <- Ecto.UUID.cast(id) do
      Repo.transaction(fn ->
        case Repo.one(from q in owned(uuid, owner), lock: "FOR UPDATE") do
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

  defp owned(id, owner),
    do: from(q in Quiz, where: q.id == ^id and q.owner_hash == ^owner_hash(owner))

  defp owner_hash(owner), do: :crypto.hash(:sha256, owner)
end
