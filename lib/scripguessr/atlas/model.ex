defmodule Scripguessr.Atlas.Model do
  @moduledoc "Atlas geometry and evidence, independent of the UI and upstream SQL."
  @kinds ~w(mention speech setting)

  def catalog(release, chapters, entries) do
    with true <- chapters != [] and Enum.all?(chapters, &valid_chapter?/1),
         true <- Enum.uniq_by(chapters, &{&1["book"], &1["chapter"]}) == chapters,
         true <- Enum.all?(entries, &valid_entry?/1) do
      chapters =
        Enum.map(chapters, &%{book: &1["book"], chapter: &1["chapter"], verses: &1["verses"]})

      entries = Enum.map(entries, &entry/1)
      by_key = Map.new(entries, &{&1.key, &1})

      if map_size(by_key) == length(entries) do
        {:ok,
         %{
           release: release,
           chapters: chapters,
           entries: entries,
           by_key: by_key,
           categories: entries |> Enum.map(& &1.category) |> Enum.uniq() |> Enum.sort(),
           books:
             Enum.map(Enum.chunk_by(chapters, & &1.book), fn rows ->
               %{name: hd(rows).book, chapters: rows}
             end)
         }}
      else
        {:error, :invalid_catalog}
      end
    else
      _ -> {:error, :invalid_catalog}
    end
  end

  def layer(catalog, entry, rows) do
    Enum.reduce_while(rows, {:ok, %{}}, fn row, {:ok, coverage} ->
      case expand_span(catalog.chapters, row) do
        {:ok, ranges} ->
          coverage =
            Enum.reduce(ranges, coverage, fn {chapter, kind, range}, acc ->
              Map.update(acc, chapter, %{kind => [range]}, fn kinds ->
                Map.update(kinds, kind, [range], &[range | &1])
              end)
            end)

          {:cont, {:ok, coverage}}

        error ->
          {:halt, error}
      end
    end)
    |> case do
      {:ok, coverage} ->
        coverage =
          Map.new(coverage, fn {chapter, kinds} ->
            {chapter, Map.new(kinds, fn {kind, ranges} -> {kind, merge_ranges(ranges)} end)}
          end)

        {:ok, %{entry: entry, coverage: coverage}}

      error ->
        error
    end
  end

  def hits(layers, chapter) do
    for layer <- layers,
        ranges = layer.coverage[chapter],
        not is_nil(ranges),
        do: %{entry: layer.entry, ranges: ranges}
  end

  def verse_hits(hits, verse) do
    Enum.filter(hits, fn hit ->
      Enum.any?(hit.ranges, fn {_kind, ranges} ->
        Enum.any?(ranges, fn {first, last} -> verse >= first and verse <= last end)
      end)
    end)
  end

  def chapter(catalog, book, number),
    do: Enum.find(catalog.chapters, &(&1.book == book and &1.chapter == number))

  defp valid_chapter?(%{"book" => book, "chapter" => chapter, "verses" => verses}),
    do:
      is_binary(book) and book != "" and is_integer(chapter) and chapter > 0 and
        is_integer(verses) and verses > 0

  defp valid_chapter?(_), do: false

  defp valid_entry?(%{
         "id" => id,
         "name" => name,
         "description" => description,
         "type" => type,
         "category" => category,
         "kinds" => kinds
       }) do
    is_integer(id) and id > 0 and is_binary(name) and name != "" and is_binary(description) and
      is_binary(type) and type != "" and is_binary(category) and category != "" and
      is_binary(kinds) and Enum.all?(String.split(kinds, ","), &(&1 in @kinds))
  end

  defp valid_entry?(_), do: false

  defp entry(row) do
    # Numeric corpus IDs are only used within a release. URLs use semantic identities.
    key =
      :crypto.hash(:sha256, Jason.encode!([row["type"], row["name"], row["description"]]))
      |> Base.url_encode64(padding: false)

    %{
      key: key,
      id: row["id"],
      name: row["name"],
      description: row["description"],
      type: row["type"],
      category: row["category"],
      kinds: String.split(row["kinds"], ",")
    }
  end

  defp expand_span(chapters, %{
         "kind" => kind,
         "first_book" => fb,
         "first_chapter" => fc,
         "first_verse" => fv,
         "last_book" => lb,
         "last_chapter" => lc,
         "last_verse" => lv
       })
       when kind in @kinds and is_integer(fv) and fv >= 0 and is_integer(lv) and lv >= 0 do
    first = Enum.find_index(chapters, &(&1.book == fb and &1.chapter == fc))
    last = Enum.find_index(chapters, &(&1.book == lb and &1.chapter == lc))

    if is_integer(first) and is_integer(last) and first <= last and
         fv <= Enum.at(chapters, first).verses and lv <= Enum.at(chapters, last).verses and
         (first != last or fv <= lv) do
      {:ok,
       for index <- first..last do
         chapter = Enum.at(chapters, index)
         from = if index == first, do: fv, else: 0
         to = if index == last, do: lv, else: chapter.verses
         {{chapter.book, chapter.chapter}, kind, {from, to}}
       end}
    else
      {:error, :invalid_span}
    end
  end

  defp expand_span(_, _), do: {:error, :invalid_span}

  defp merge_ranges(ranges) do
    ranges
    |> Enum.sort()
    |> Enum.reduce([], fn
      {first, last}, [{previous_first, previous_last} | rest] when first <= previous_last + 1 ->
        [{previous_first, max(last, previous_last)} | rest]

      range, acc ->
        [range | acc]
    end)
    |> Enum.reverse()
  end
end
