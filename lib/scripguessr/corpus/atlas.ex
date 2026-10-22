defmodule Scripguessr.Corpus.Atlas do
  @moduledoc "Corpus-backed Atlas catalog, passage ranges, and chapter text."
  alias Scripguessr.{Corpus, Atlas.Model}
  @page_size 5_000
  @max_pages 40

  def catalog do
    with {:ok, response} <- Corpus.query(chapters_sql()),
         false <- response.truncated,
         {:ok, entries} <- pages(&catalog_sql/1, response.release),
         {:ok, catalog} <- Model.catalog(response.release, response.rows, entries) do
      {:ok, catalog}
    else
      {:error, _} = error -> error
      _ -> {:error, :invalid_catalog}
    end
  end

  def layer(catalog, key) do
    case catalog.by_key[key] do
      nil ->
        {:error, :unknown_entry}

      entry ->
        with {:ok, rows} <- pages(&layer_sql(entry.id, &1), catalog.release),
             do: Model.layer(catalog, entry, rows)
    end
  end

  def chapter(catalog, book, number) do
    if Model.chapter(catalog, book, number) do
      Scripguessr.Corpus.Chapter.read(book, number, catalog.release)
    else
      {:error, :unknown_chapter}
    end
  end

  defp pages(sql, release, page \\ 0, acc \\ [])
  defp pages(_sql, _release, page, _acc) when page >= @max_pages, do: {:error, :too_many_pages}

  defp pages(sql, release, page, acc) do
    offset = Enum.reduce(acc, 0, &(length(&1) + &2))

    with {:ok, response} <- Corpus.query(sql.(offset)),
         true <- response.release == release || {:error, :release_changed},
         true <- length(response.rows) <= @page_size,
         true <- !response.truncated or response.rows != [] do
      if response.truncated or length(response.rows) == @page_size do
        pages(sql, release, page + 1, [response.rows | acc])
      else
        {:ok, acc |> Enum.reverse() |> List.flatten() |> Enum.concat(response.rows)}
      end
    else
      {:error, _} = error -> error
      _ -> {:error, :invalid_page}
    end
  end

  defp chapters_sql do
    """
    /* atlas:chapters */
    select b.name as book, c.number as chapter, max(v.number) as verses
    from chapter c join book b on b.id=c.book_id join edition e on e.id=c.edition_id
    join edition_book eb on eb.edition_id=e.id and eb.book_id=b.id
    join verse v on v.chapter_id=c.id
    where e.name='#{Corpus.edition()}'
    group by c.id order by eb.position,c.number
    """
  end

  defp catalog_sql(offset) do
    """
    /* atlas:catalog */
    with recursive roots(id,root) as (
      select id,name from entity_type where parent_id is null
      union all select t.id,r.root from entity_type t join roots r on t.parent_id=r.id
    ), annotations(entity_id,first_word_id,kind) as (
      select entity_id,first_word_id,'mention' from mention
      union all select ss.entity_id,s.first_word_id,'speech' from speech s join speech_speaker ss on ss.speech_id=s.id
      union all select place_id,first_word_id,'setting' from setting
    )
    select x.id,x.name,x.description,t.name as type,r.root as category,group_concat(distinct a.kind) as kinds
    from annotations a join word w on w.id=a.first_word_id join verse v on v.id=w.verse_id
    join chapter c on c.id=v.chapter_id join edition e on e.id=c.edition_id
    join entity x on x.id=a.entity_id join entity_type t on t.id=x.entity_type_id join roots r on r.id=t.id
    where e.name='#{Corpus.edition()}'
    group by x.id order by r.root,x.name,x.description,x.id
    limit #{@page_size} offset #{offset}
    """
  end

  defp layer_sql(id, offset) when is_integer(id) and id > 0 do
    """
    /* atlas:layer entity=#{id} */
    with spans(first_word_id,last_word_id,kind) as (
      select first_word_id,last_word_id,'mention' from mention where entity_id=#{id}
      union all select s.first_word_id,s.last_word_id,'speech' from speech s join speech_speaker ss on ss.speech_id=s.id where ss.entity_id=#{id}
      union all select first_word_id,last_word_id,'setting' from setting where place_id=#{id}
    )
    select distinct a.kind,fb.name as first_book,fc.number as first_chapter,coalesce(fv.number,0) as first_verse,
      lb.name as last_book,lc.number as last_chapter,coalesce(lv.number,0) as last_verse
    from spans a join word fw on fw.id=a.first_word_id join word lw on lw.id=a.last_word_id
    join verse fv on fv.id=fw.verse_id join chapter fc on fc.id=fv.chapter_id join book fb on fb.id=fc.book_id
    join verse lv on lv.id=lw.verse_id join chapter lc on lc.id=lv.chapter_id join book lb on lb.id=lc.book_id
    join edition e on e.id=fc.edition_id
    where e.name='#{Corpus.edition()}' and lc.edition_id=fc.edition_id and fw.sequence<=lw.sequence
    order by a.kind,fb.name,fc.number,first_verse,lb.name,lc.number,last_verse
    limit #{@page_size} offset #{offset}
    """
  end
end
