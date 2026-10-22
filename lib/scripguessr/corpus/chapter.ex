defmodule Scripguessr.Corpus.Chapter do
  @moduledoc "Chapter text shared by the game and Atlas, checked against the requested corpus release."
  alias Scripguessr.Corpus

  def read(book, number, release) when is_binary(book) and is_integer(number) and number > 0 do
    with {:ok, response} <- Corpus.query(chapter_sql(book, number)),
         true <- response.release == release || {:error, :release_changed},
         false <- response.truncated,
         true <- response.rows != [] and Enum.all?(response.rows, &valid_verse?/1),
         true <- Enum.uniq_by(response.rows, & &1["verse"]) == response.rows do
      {:ok, response.rows}
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
