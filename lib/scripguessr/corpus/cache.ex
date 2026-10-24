defmodule Scripguessr.Corpus.Cache do
  @moduledoc "Shared game metadata with independent, bounded game preparation tasks."
  alias Scripguessr.Corpus
  alias Corpus.RequestCache

  def child_spec(_),
    do: %{id: __MODULE__, start: {RequestCache, :start_link, [[name: __MODULE__]]}}

  def clear, do: RequestCache.clear(__MODULE__)
  def prepare, do: prepare(1)

  defp prepare(retries) do
    with {:ok, catalog} <- RequestCache.fetch(__MODULE__, :catalog, &Corpus.catalog/0),
         # Every game gets its own selection; only metadata requests are shared.
         {:ok, snapshot} <-
           RequestCache.fetch(__MODULE__, make_ref(), fn -> Corpus.prepare(catalog) end, ttl: 0) do
      {:ok, snapshot}
    else
      {:error, :release_changed} when retries > 0 ->
        RequestCache.invalidate(__MODULE__, :catalog)
        prepare(retries - 1)

      error ->
        error
    end
  end
end
