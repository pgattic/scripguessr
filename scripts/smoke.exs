Application.ensure_all_started(:req)

with {:ok, catalog} <- Scripguessr.Corpus.catalog(),
     {:ok, snapshot} <- Scripguessr.Corpus.prepare(catalog),
     5 <- length(snapshot["rounds"]) do
  IO.puts(
    "Corpus smoke test passed: #{catalog.release}, #{length(catalog.references)} eligible verses, five fetched prompts."
  )
else
  error -> raise "Corpus smoke test failed: #{inspect(error)}"
end
