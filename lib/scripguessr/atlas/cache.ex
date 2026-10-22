defmodule Scripguessr.Atlas.Cache do
  @moduledoc "Independent, bounded Atlas cache. Cache keys include the corpus release."
  use GenServer
  alias Scripguessr.Corpus.Atlas
  @ttl :timer.minutes(15)
  @limit 160

  def start_link(opts), do: GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  def catalog, do: GenServer.call(__MODULE__, :catalog, :infinity)
  def layers(catalog, keys), do: GenServer.call(__MODULE__, {:layers, catalog, keys}, :infinity)

  def chapter(catalog, book, number),
    do: GenServer.call(__MODULE__, {:chapter, catalog, book, number}, :infinity)

  def clear, do: GenServer.call(__MODULE__, :clear)
  @impl true
  def init(_), do: {:ok, %{catalog: nil, entries: %{}}}
  @impl true
  def handle_call(:clear, _from, _state), do: {:reply, :ok, %{catalog: nil, entries: %{}}}

  def handle_call(:catalog, _from, state) do
    case state.catalog do
      {catalog, expires} ->
        if expires > now(), do: {:reply, {:ok, catalog}, state}, else: load_catalog(state)

      nil ->
        load_catalog(state)
    end
  end

  def handle_call({:layers, catalog, keys}, _from, state) do
    {result, state} =
      Enum.reduce_while(keys, {{:ok, []}, state}, fn key, {{:ok, layers}, state} ->
        {result, state} =
          cached(state, {:layer, catalog.release, key}, fn -> Atlas.layer(catalog, key) end)

        case result do
          {:ok, layer} -> {:cont, {{:ok, layers ++ [layer]}, state}}
          error -> {:halt, {error, state}}
        end
      end)

    {:reply, result, state}
  end

  def handle_call({:chapter, catalog, book, number}, _from, state) do
    {result, state} =
      cached(state, {:chapter, catalog.release, book, number}, fn ->
        Atlas.chapter(catalog, book, number)
      end)

    {:reply, result, state}
  end

  defp load_catalog(state) do
    result =
      case Atlas.catalog() do
        {:error, :release_changed} -> Atlas.catalog()
        result -> result
      end

    case result do
      {:ok, catalog} -> {:reply, result, %{state | catalog: {catalog, now() + @ttl}}}
      error -> {:reply, error, state}
    end
  end

  defp cached(state, key, fetch) do
    case state.entries[key] do
      {value, expires} ->
        if expires > now(), do: {{:ok, value}, state}, else: fetch_and_store(state, key, fetch)

      nil ->
        fetch_and_store(state, key, fetch)
    end
  end

  defp fetch_and_store(state, key, fetch) do
    case fetch.() do
      {:ok, value} = result ->
        entries =
          if map_size(state.entries) >= @limit do
            {oldest, _} = Enum.min_by(state.entries, fn {_, {_, expires}} -> expires end)
            Map.delete(state.entries, oldest)
          else
            state.entries
          end

        {result, %{state | entries: Map.put(entries, key, {value, now() + @ttl})}}

      error ->
        {error, state}
    end
  end

  # BEAM monotonic timestamps may be negative; shifting is unnecessary.
  defp now, do: System.monotonic_time(:millisecond)
end
