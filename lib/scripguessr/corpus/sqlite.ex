defmodule Scripguessr.Corpus.SQLite do
  @moduledoc "Read-only access to the corpus release bundled with the application."
  @behaviour Scripguessr.Corpus.Client
  use Supervisor

  @pool __MODULE__.Pool
  @release_key {__MODULE__, :release}
  @timeout 10_000

  def start_link(opts \\ []), do: Supervisor.start_link(__MODULE__, opts, name: __MODULE__)

  @impl true
  def init(opts) do
    path = Keyword.get_lazy(opts, :database, &database_path/0)
    # Fail startup for a missing bundle rather than create an empty database.
    unless File.regular?(path), do: raise("Corpus database is missing: #{path}")
    release = path |> Path.dirname() |> Path.join("release.txt") |> File.read!() |> String.trim()
    if release == "", do: raise("Corpus release identifier is empty: #{path}")
    :persistent_term.put(@release_key, release)

    Supervisor.init(
      [{Exqlite, name: @pool, database: path, mode: :readonly, pool_size: 4}],
      strategy: :one_for_one
    )
  end

  @impl true
  def query(sql) do
    case Exqlite.query(@pool, sql, [], timeout: @timeout) do
      {:ok, %{columns: columns, rows: rows}} ->
        {:ok,
         %{
           release: :persistent_term.get(@release_key),
           rows: Enum.map(rows, &(columns |> Enum.zip(&1) |> Map.new())),
           truncated: false
         }}

      {:error, error} ->
        {:error, {:sqlite, Exception.message(error)}}
    end
  rescue
    DBConnection.ConnectionError -> {:error, :unavailable}
  end

  defp database_path, do: Application.fetch_env!(:scripguessr, :corpus_path)
end
