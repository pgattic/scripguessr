defmodule Scripguessr.Corpus.Cache do
  @moduledoc "Single-flight metadata cache; no corpus request is needed to play prepared games."
  use GenServer
  alias Scripguessr.Corpus
  @ttl_ms :timer.minutes(15)
  def start_link(opts), do: GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  def prepare, do: GenServer.call(__MODULE__, :prepare, :infinity)
  def clear, do: GenServer.call(__MODULE__, :clear)
  @impl true
  def init(_), do: {:ok, nil}
  @impl true
  def handle_call(:clear, _from, _state), do: {:reply, :ok, nil}

  def handle_call(:prepare, _from, state) do
    {reply, state} = prepare(state, 1)
    {:reply, reply, state}
  end

  defp prepare(state, retries) do
    with {:ok, catalog, cached_at} <- catalog(state),
         {:ok, snapshot} <- Corpus.prepare(catalog) do
      {{:ok, snapshot}, {catalog, cached_at}}
    else
      {:error, :release_changed} when retries > 0 -> prepare(nil, retries - 1)
      {:error, reason} -> {{:error, reason}, nil}
    end
  end

  defp catalog({catalog, cached_at}) do
    if System.monotonic_time(:millisecond) - cached_at < @ttl_ms,
      do: {:ok, catalog, cached_at},
      else: catalog(nil)
  end

  defp catalog(nil) do
    case Corpus.catalog() do
      {:ok, catalog} -> {:ok, catalog, System.monotonic_time(:millisecond)}
      error -> error
    end
  end
end
