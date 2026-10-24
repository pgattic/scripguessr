defmodule Scripguessr.Atlas.Cache do
  @moduledoc "Bounded Atlas catalog and layer cache, with release-scoped keys."
  alias Scripguessr.Corpus.{Atlas, RequestCache}

  def child_spec(_),
    do: %{id: __MODULE__, start: {RequestCache, :start_link, [[name: __MODULE__]]}}

  def clear, do: RequestCache.clear(__MODULE__)

  def catalog do
    RequestCache.fetch(__MODULE__, :catalog, fn ->
      case Atlas.catalog() do
        {:error, :release_changed} -> Atlas.catalog()
        result -> result
      end
    end)
  end

  def layers(catalog, keys) do
    Enum.reduce_while(keys, {:ok, []}, fn key, {:ok, layers} ->
      case RequestCache.fetch(__MODULE__, {:layer, catalog.release, key}, fn ->
             Atlas.layer(catalog, key)
           end) do
        {:ok, layer} -> {:cont, {:ok, [layer | layers]}}
        error -> {:halt, error}
      end
    end)
    |> case do
      {:ok, layers} -> {:ok, Enum.reverse(layers)}
      error -> error
    end
  end

  def chapter(catalog, book, number), do: Atlas.chapter(catalog, book, number)
end
