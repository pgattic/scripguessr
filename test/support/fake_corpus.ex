defmodule Scripguessr.Corpus.Fake do
  @behaviour Scripguessr.Corpus.Client
  def start_link,
    do:
      Agent.start_link(fn -> %{mode: :normal, release: "fixture-v1", calls: [], block: nil} end,
        name: __MODULE__
      )

  def reset(mode \\ :normal),
    do:
      Agent.update(__MODULE__, fn _ ->
        %{mode: mode, release: "fixture-v1", calls: [], block: nil}
      end)

  def calls, do: Agent.get(__MODULE__, & &1.calls)

  def block_next(pattern, notify),
    do: Agent.update(__MODULE__, &%{&1 | block: {pattern, notify}})

  def query(sql) do
    {result, notify} =
      Agent.get_and_update(__MODULE__, fn state ->
        state = %{state | calls: state.calls ++ [sql]}

        {notify, state} =
          case state.block do
            {pattern, pid} ->
              if String.contains?(sql, pattern),
                do: {pid, %{state | block: nil}},
                else: {nil, state}

            nil ->
              {nil, state}
          end

        {result, state} =
          case state.mode do
            :down -> {{:error, :unavailable}, state}
            _ -> response(sql, state)
          end

        {{result, notify}, state}
      end)

    if notify do
      send(notify, {:corpus_blocked, sql, self()})

      receive do
        :release_corpus -> :ok
      end
    end

    result
  end

  defp response(sql, state) do
    if String.contains?(sql, "atlas:") do
      Scripguessr.AtlasFixture.response(sql, state)
    else
      game_response(sql, state)
    end
  end

  defp game_response(sql, state) do
    is_text = String.contains?(sql, "group_concat")

    cond do
      is_text and state.mode == :release_once ->
        state = %{state | mode: :normal, release: "fixture-v2"}
        {{:ok, %{release: state.release, rows: text_rows(sql), truncated: false}}, state}

      is_text and state.mode == :release_always ->
        {{:ok, %{release: "other", rows: text_rows(sql), truncated: false}}, state}

      is_text ->
        rows = if state.mode == :missing_text, do: [], else: text_rows(sql)
        {{:ok, %{release: state.release, rows: rows, truncated: false}}, state}

      String.contains?(sql, "exists") ->
        rows = references(state.mode)
        [_, offset] = Regex.run(~r/offset (\d+)/, sql)
        rows = Enum.slice(rows, String.to_integer(offset), 1000)
        {{:ok, %{release: state.release, rows: rows, truncated: false}}, state}

      true ->
        chapters =
          for {book, count} <- [{"1 Nephi", 22}, {"2 Nephi", 33}],
              chapter <- 1..count,
              do: %{"book" => book, "chapter" => chapter}

        {{:ok, %{release: state.release, rows: chapters, truncated: false}}, state}
    end
  end

  defp references(:paginated),
    do: for(n <- 1..1001, do: %{"book" => "1 Nephi", "chapter" => 1, "verse" => n})

  defp references(_), do: for(n <- 1..8, do: %{"book" => "1 Nephi", "chapter" => n, "verse" => 1})

  defp text_rows(sql) do
    Regex.scan(~r/\(b.name = '([^']+)' and c.number = (\d+) and v.number = (\d+)\)/, sql)
    |> Enum.map(fn [_, book, chapter, verse] ->
      %{
        "book" => book,
        "chapter" => String.to_integer(chapter),
        "verse" => String.to_integer(verse),
        "text" => "A short invented passage for round #{chapter}."
      }
    end)
  end
end
