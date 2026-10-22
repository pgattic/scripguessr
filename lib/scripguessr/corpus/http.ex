defmodule Scripguessr.Corpus.HTTP do
  @behaviour Scripguessr.Corpus.Client
  @impl true
  def query(sql) do
    url = Application.fetch_env!(:scripguessr, :corpus_url)
    options = Application.get_env(:scripguessr, :corpus_http_options, [])

    case Req.get(
           [
             url: url,
             params: [q: sql],
             receive_timeout: 10_000,
             connect_options: [timeout: 5_000],
             retry: false
           ] ++ options
         ) do
      {:ok,
       %{status: 200, body: %{"release" => release, "rows" => rows, "truncated" => truncated}}}
      when is_binary(release) and is_list(rows) and is_boolean(truncated) ->
        if release != "" and Enum.all?(rows, &is_map/1) do
          {:ok, %{release: release, rows: rows, truncated: truncated}}
        else
          {:error, :invalid_response}
        end

      {:ok, %{status: status}} when status != 200 ->
        {:error, {:http, status}}

      {:ok, _} ->
        {:error, :invalid_response}

      {:error, _} ->
        {:error, :unavailable}
    end
  end
end
