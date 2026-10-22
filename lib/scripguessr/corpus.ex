defmodule Scripguessr.Corpus do
  @moduledoc "The only boundary that knows the corpus SQL schema."
  @edition "Book of Mormon (2013)"
  @page_size 1_000
  @max_pages 100

  def edition, do: @edition
  def query(sql), do: Application.fetch_env!(:scripguessr, :corpus_client).query(sql)

  def catalog do
    with {:ok, metadata} <- query(metadata_sql()),
         false <- metadata.truncated,
         {:ok, chapters} <- validate_chapters(metadata.rows),
         {:ok, references} <- references(metadata.release, 0, []),
         true <- length(references) >= 5,
         true <- Enum.all?(references, &valid_reference?(&1, chapters)) do
      {:ok, %{release: metadata.release, chapters: chapters, references: references}}
    else
      {:error, _} = error -> error
      _ -> {:error, :invalid_catalog}
    end
  end

  def prepare(catalog) do
    selected = Enum.take_random(catalog.references, 5)

    with {:ok, response} <- query(text_sql(selected)),
         true <- response.release == catalog.release || {:error, :release_changed},
         false <- response.truncated,
         {:ok, rounds} <- validate_text(response.rows, selected) do
      {:ok,
       %{
         "edition" => @edition,
         "release" => catalog.release,
         "chapters" => catalog.chapters,
         "rounds" => rounds,
         "current" => 0,
         "finished" => false
       }}
    else
      {:error, _} = error -> error
      _ -> {:error, :invalid_text}
    end
  end

  defp references(_release, page, _acc) when page >= @max_pages, do: {:error, :too_many_pages}

  defp references(release, page, acc) do
    with {:ok, response} <- query(references_sql(page)),
         true <- response.release == release || {:error, :release_changed} do
      rows = acc ++ response.rows

      if response.truncated or length(response.rows) == @page_size do
        references(release, page + 1, rows)
      else
        if Enum.uniq(rows) == rows, do: {:ok, rows}, else: {:error, :duplicate_references}
      end
    else
      {:error, _} = error -> error
    end
  end

  defp validate_chapters(rows) do
    valid =
      rows != [] and
        Enum.all?(rows, fn
          %{"book" => book, "chapter" => chapter} ->
            is_binary(book) and book != "" and is_integer(chapter) and chapter > 0

          _ ->
            false
        end) and Enum.uniq(rows) == rows

    if valid, do: {:ok, rows}, else: {:error, :invalid_chapters}
  end

  defp valid_reference?(%{"book" => book, "chapter" => chapter, "verse" => verse} = ref, chapters) do
    map_size(ref) == 3 and is_integer(verse) and verse > 0 and
      %{"book" => book, "chapter" => chapter} in chapters
  end

  defp valid_reference?(_, _), do: false

  defp validate_text(rows, selected) do
    by_ref = Map.new(rows, fn row -> {Map.take(row, ["book", "chapter", "verse"]), row} end)

    if length(rows) == 5 and map_size(by_ref) == 5 and
         Enum.all?(selected, fn ref ->
           case by_ref[ref] do
             %{"text" => text} when is_binary(text) -> String.trim(text) != ""
             _ -> false
           end
         end) do
      {:ok,
       Enum.map(selected, fn ref ->
         Map.take(by_ref[ref], ["book", "chapter", "verse", "text"])
       end)}
    else
      {:error, :invalid_text}
    end
  end

  defp metadata_sql do
    """
    select b.name as book, c.number as chapter
    from chapter c join book b on b.id = c.book_id
    join edition e on e.id = c.edition_id
    join edition_book eb on eb.edition_id = e.id and eb.book_id = b.id
    where e.name = '#{@edition}'
    order by eb.position, c.number
    """
  end

  defp references_sql(page) do
    """
    select b.name as book, c.number as chapter, v.number as verse
    from verse v join chapter c on c.id = v.chapter_id
    join book b on b.id = c.book_id join edition e on e.id = c.edition_id
    join edition_book eb on eb.edition_id = e.id and eb.book_id = b.id
    where e.name = '#{@edition}' and v.number > 0
      and exists (select 1 from word w where w.verse_id = v.id and trim(w.text) <> '')
    order by eb.position, c.number, v.number
    limit #{@page_size} offset #{page * @page_size}
    """
  end

  defp text_sql(references) do
    # Values originate in validated corpus metadata, never raw browser input.
    conditions =
      Enum.map_join(references, " or ", fn ref ->
        "(b.name = '#{String.replace(ref["book"], "'", "''")}' and c.number = #{ref["chapter"]} and v.number = #{ref["verse"]})"
      end)

    """
    select b.name as book, c.number as chapter, v.number as verse,
      group_concat(w.before || w.text || w.after, '' order by w.position) as text
    from verse v join word w on w.verse_id = v.id
    join chapter c on c.id = v.chapter_id join book b on b.id = c.book_id
    join edition e on e.id = c.edition_id
    where e.name = '#{@edition}' and (#{conditions})
    group by v.id
    """
  end
end
