defmodule Scripguessr.Corpus.Chapter do
  @moduledoc "Current chapter text shared by games, matches, and the Atlas."
  alias Scripguessr.Corpus
  alias Corpus.RequestCache

  def child_spec(_),
    do: %{id: __MODULE__, start: {RequestCache, :start_link, [[name: __MODULE__]]}}

  def clear, do: RequestCache.clear(__MODULE__)

  def read(book, number) when is_binary(book) and is_integer(number) and number > 0 do
    RequestCache.fetch(__MODULE__, {Corpus.edition(), book, number}, fn -> fetch(book, number) end)
  end

  defp fetch(book, number) do
    with {:ok, response} <- Corpus.query(chapter_sql(book, number)),
         false <- response.truncated,
         true <- response.rows != [] and Enum.all?(response.rows, &valid_verse?/1),
         true <- Enum.uniq_by(response.rows, & &1["verse"]) == response.rows do
      {:ok, %{verses: response.rows, release: response.release}}
    else
      {:error, _} = error -> error
      _ -> {:error, :invalid_chapter}
    end
  end

  defp valid_verse?(%{"verse" => number, "text" => text}),
    do: is_integer(number) and number >= 0 and is_binary(text) and String.trim(text) != ""

  defp valid_verse?(_), do: false

  defp chapter_sql(book, number) do
    """
    /* atlas:reader */
    select coalesce(v.number,0) as verse,group_concat(w.before || w.text || w.after,'' order by w.position) as text
    from verse v join word w on w.verse_id=v.id join chapter c on c.id=v.chapter_id
    join book b on b.id=c.book_id join edition e on e.id=c.edition_id
    where e.name='#{Corpus.edition()}' and b.name='#{String.replace(book, "'", "''")}' and c.number=#{number}
    group by v.id order by v.number
    """
  end
end
