defmodule Scripguessr.AtlasFixture do
  def chapters do
    [
      %{"book" => "1 Nephi", "chapter" => 1, "verses" => 3},
      %{"book" => "1 Nephi", "chapter" => 2, "verses" => 3},
      %{"book" => "Jacob", "chapter" => 1, "verses" => 3}
    ]
  end

  def entries do
    [
      entry(1, "Nephi", "Founder of the Nephite people.", "Person", "Person", "mention,speech"),
      entry(2, "Nephi", "A later prophet descended from Nephi.", "Person", "Person", "mention"),
      entry(3, "Faith", "Trust in God.", "Topic", "Topic", "mention"),
      entry(4, "Jerusalem", "A city.", "City", "Place", "setting"),
      entry(
        5,
        "A newly annotated entry",
        "A category discovered upstream.",
        "New category",
        "New category",
        "mention"
      )
    ]
  end

  defp entry(id, name, description, type, category, kinds),
    do: %{
      "id" => id,
      "name" => name,
      "description" => description,
      "type" => type,
      "category" => category,
      "kinds" => kinds
    }

  def catalog, do: Scripguessr.Atlas.Model.catalog("fixture-v1", chapters(), entries())

  def key(id) do
    {:ok, catalog} = catalog()
    Enum.find(catalog.entries, &(&1.id == id)).key
  end

  def spans(1),
    do: [
      span("mention", "1 Nephi", 1, 1, "1 Nephi", 1, 1),
      span("speech", "1 Nephi", 1, 2, "1 Nephi", 2, 2)
    ]

  def spans(2), do: [span("mention", "Jacob", 1, 1, "Jacob", 1, 1)]
  def spans(3), do: [span("mention", "1 Nephi", 2, 2, "Jacob", 1, 2)]
  def spans(4), do: [span("setting", "1 Nephi", 1, 1, "1 Nephi", 1, 3)]
  def spans(_), do: []

  def span(kind, fb, fc, fv, lb, lc, lv),
    do: %{
      "kind" => kind,
      "first_book" => fb,
      "first_chapter" => fc,
      "first_verse" => fv,
      "last_book" => lb,
      "last_chapter" => lc,
      "last_verse" => lv
    }

  def response(sql, state) do
    cond do
      String.contains?(sql, "atlas:chapters") ->
        reply(chapters(), state)

      String.contains?(sql, "atlas:catalog") ->
        state =
          if state.mode == :atlas_catalog_change,
            do: %{state | mode: :normal, release: "fixture-v2"},
            else: state

        rows =
          if state.mode == :atlas_shifted,
            do: Enum.map(entries(), &Map.update!(&1, "id", fn id -> id + 100 end)),
            else: entries()

        reply(rows, state)

      String.contains?(sql, "atlas:layer") ->
        [_, id] = Regex.run(~r/entity=(\d+)/, sql)

        state =
          case state.mode do
            :atlas_layer_change -> %{state | mode: :normal, release: "fixture-v2"}
            :atlas_layer_renumber -> %{state | mode: :atlas_shifted, release: "fixture-v2"}
            :atlas_layer_always_change -> %{state | release: state.release <> "-next"}
            _ -> state
          end

        id = String.to_integer(id)
        rows = spans(if(state.mode == :atlas_shifted && id > 100, do: id - 100, else: id))

        case state.mode do
          :atlas_bad_span ->
            reply([%{"kind" => "invalid"}], state)

          :atlas_paged ->
            if String.contains?(sql, "offset 0"),
              do: reply(Enum.take(rows, 1), state, true),
              else: reply(Enum.drop(rows, 1), state)

          _ ->
            reply(rows, state)
        end

      String.contains?(sql, "atlas:reader") ->
        rows =
          for number <- 1..3,
              do: %{"verse" => number, "text" => "Invented study passage #{number}."}

        if state.mode == :atlas_bad_reader,
          do: reply([%{"verse" => 1, "text" => ""}], state),
          else: reply(rows, state)
    end
  end

  defp reply(rows, state, truncated \\ false),
    do: {{:ok, %{release: state.release, rows: rows, truncated: truncated}}, state}
end
