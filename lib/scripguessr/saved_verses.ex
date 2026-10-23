defmodule Scripguessr.SavedVerses do
  import Ecto.Query
  alias Scripguessr.{Corpus, Repo}
  alias Scripguessr.SavedVerses.Verse

  def list(user_id) do
    Repo.all(
      from v in Verse, where: v.user_id == ^user_id, order_by: [desc: v.inserted_at, desc: v.id]
    )
  end

  def key(verse), do: {verse["book"], verse["chapter"], verse["verse"]}

  def keys(nil), do: MapSet.new()
  def keys(user), do: list(user.id) |> MapSet.new(&{&1.book, &1.chapter, &1.verse})

  # Callers supply corpus text already loaded on the server, never browser text.
  def save(user_id, verse, release) do
    %Verse{
      user_id: user_id,
      edition: Corpus.edition(),
      book: verse["book"],
      chapter: verse["chapter"],
      verse: verse["verse"],
      text: verse["text"],
      release: release
    }
    |> Repo.insert(
      on_conflict: :nothing,
      conflict_target: [:user_id, :edition, :book, :chapter, :verse]
    )
  end

  def remove(user_id, {book, chapter, verse}) do
    Repo.delete_all(
      from v in Verse,
        where:
          v.user_id == ^user_id and v.edition == ^Corpus.edition() and v.book == ^book and
            v.chapter == ^chapter and v.verse == ^verse
    )

    :ok
  end

  def remove_by_id(user_id, id) do
    case Ecto.UUID.cast(id) do
      {:ok, id} -> Repo.delete_all(from v in Verse, where: v.user_id == ^user_id and v.id == ^id)
      :error -> {0, nil}
    end
  end
end
