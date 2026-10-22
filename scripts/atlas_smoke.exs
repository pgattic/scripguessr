Application.ensure_all_started(:req)
alias Scripguessr.Corpus.Atlas
alias Scripguessr.Atlas.Model

with {:ok, catalog} <- Atlas.catalog(),
     entry when not is_nil(entry) <- Enum.find(catalog.entries, &(&1.name == "Faith")),
     {:ok, layer} <- Atlas.layer(catalog, entry.key),
     true <- map_size(layer.coverage) > 0,
     {{book, number}, _ranges} <- Enum.at(layer.coverage, 0),
     {:ok, verses} <- Atlas.chapter(catalog, book, number),
     true <- verses != [],
     [_hit] <- Model.hits([layer], {book, number}) do
  IO.puts(
    "Atlas smoke test passed: #{catalog.release}, #{length(catalog.books)} books, #{length(catalog.chapters)} chapters, #{length(catalog.entries)} entries; Faith highlights #{map_size(layer.coverage)} chapters; chapter reading works."
  )
else
  error -> raise "Atlas smoke test failed: #{inspect(error)}"
end
