defmodule Scripguessr.Atlas.ModelTest do
  use ExUnit.Case, async: true
  alias Scripguessr.Atlas.Model
  alias Scripguessr.AtlasFixture, as: Fixture

  test "catalog preserves book order and discovers new categories without UI definitions" do
    {:ok, catalog} = Fixture.catalog()
    assert Enum.map(catalog.books, & &1.name) == ["1 Nephi", "Jacob"]
    assert "New category" in catalog.categories
    people = Enum.filter(catalog.entries, &(&1.name == "Nephi"))
    assert length(people) == 2
    assert length(Enum.uniq_by(people, & &1.key)) == 2
    shifted = Enum.map(Fixture.entries(), &Map.update!(&1, "id", fn n -> n + 1000 end))
    {:ok, next} = Model.catalog("next-release", Fixture.chapters(), shifted)
    assert Enum.map(catalog.entries, & &1.key) == Enum.map(next.entries, & &1.key)
  end

  test "speech highlights every intersected chapter, not only its start" do
    {:ok, catalog} = Fixture.catalog()
    entry = catalog.by_key[Fixture.key(1)]
    {:ok, layer} = Model.layer(catalog, entry, Fixture.spans(1))
    assert layer.coverage[{"1 Nephi", 1}]["speech"] == [{2, 3}]
    assert layer.coverage[{"1 Nephi", 2}]["speech"] == [{0, 2}]
    assert Model.hits([layer], {"Jacob", 1}) == []
    [hit] = Model.hits([layer], {"1 Nephi", 2})
    assert Model.verse_hits([hit], 2) == [hit]
    assert Model.verse_hits([hit], 3) == []
  end

  test "cross-book spans and overlapping evidence preserve exact verse boundaries" do
    {:ok, catalog} = Fixture.catalog()
    entry = catalog.by_key[Fixture.key(3)]
    {:ok, layer} = Model.layer(catalog, entry, Fixture.spans(3))
    assert Map.keys(layer.coverage) |> Enum.sort() == [{"1 Nephi", 2}, {"Jacob", 1}]
    assert layer.coverage[{"1 Nephi", 2}]["mention"] == [{2, 3}]
    assert layer.coverage[{"Jacob", 1}]["mention"] == [{0, 2}]

    rows = [
      Fixture.span("mention", "1 Nephi", 1, 1, "1 Nephi", 1, 1),
      Fixture.span("mention", "1 Nephi", 1, 3, "1 Nephi", 1, 3)
    ]

    {:ok, layer} = Model.layer(catalog, entry, rows)
    assert layer.coverage[{"1 Nephi", 1}]["mention"] == [{1, 1}, {3, 3}]
    assert Model.verse_hits(Model.hits([layer], {"1 Nephi", 1}), 2) == []
  end

  test "rejects malformed, reversed, and unknown passage spans" do
    {:ok, catalog} = Fixture.catalog()
    entry = hd(catalog.entries)

    for span <- [
          %{},
          Fixture.span("mention", "Jacob", 1, 1, "1 Nephi", 1, 1),
          Fixture.span("mention", "Missing", 1, 1, "Jacob", 1, 1),
          Fixture.span("speech", "1 Nephi", 1, 4, "1 Nephi", 1, 4)
        ] do
      assert {:error, :invalid_span} = Model.layer(catalog, entry, [span])
    end
  end
end
