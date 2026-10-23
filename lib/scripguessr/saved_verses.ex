defmodule Scripguessr.SavedVerses do
  import Ecto.Query
  alias Scripguessr.{Corpus, Repo}
  alias Scripguessr.SavedVerses.Verse
  @page_size 20

  def page(user_id, number \\ 1) when is_integer(number) and number > 0 do
    entries =
      Repo.all(
        from v in Verse,
          where: v.user_id == ^user_id,
          order_by: [desc: v.inserted_at, desc: v.id],
          limit: ^(@page_size + 1),
          offset: ^((number - 1) * @page_size)
      )

    %{
      entries: Enum.take(entries, @page_size),
      number: number,
      more?: length(entries) > @page_size
    }
  end

  def key(verse), do: {verse["book"], verse["chapter"], verse["verse"]}

  def keys(nil), do: MapSet.new()

  def keys(user) do
    Repo.all(
      from v in Verse,
        where: v.user_id == ^user.id and v.edition == ^Corpus.edition(),
        select: {v.book, v.chapter, v.verse}
    )
    |> MapSet.new()
  end

  def set_saved(user_id, verse, release, true) do
    case save(user_id, verse, release) do
      {:ok, _} -> {:ok, true}
      {:error, changeset} -> {:error, changeset}
    end
  end

  def set_saved(user_id, verse, _release, false) do
    :ok = remove(user_id, key(verse))
    {:ok, false}
  end

  # Callers supply corpus text already loaded on the server, never browser text.
  def save(user_id, verse, release) do
    %Verse{}
    |> Verse.changeset(%{
      user_id: user_id,
      edition: Corpus.edition(),
      book: verse["book"],
      chapter: verse["chapter"],
      verse: verse["verse"],
      text: verse["text"],
      release: release
    })
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
