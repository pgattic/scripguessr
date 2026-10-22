defmodule Scripguessr.AtlasTest do
  use ExUnit.Case, async: false
  alias Scripguessr.Corpus.{Atlas, Fake}
  alias Scripguessr.Atlas.Cache
  alias Scripguessr.AtlasFixture, as: Fixture

  setup do
    Fake.reset()
    Cache.clear()
    :ok
  end

  test "caches catalog, selected layers, and chapter text independently of quiz data" do
    {:ok, catalog} = Cache.catalog()
    assert {:ok, [layer]} = Cache.layers(catalog, [Fixture.key(1)])
    assert map_size(layer.coverage) == 2
    assert {:ok, verses} = Cache.chapter(catalog, "1 Nephi", 1)
    before = Fake.calls()
    assert {:ok, ^catalog} = Cache.catalog()
    assert {:ok, [^layer]} = Cache.layers(catalog, [Fixture.key(1)])
    assert {:ok, ^verses} = Cache.chapter(catalog, "1 Nephi", 1)
    assert before == Fake.calls()
    Fake.reset(:down)
    assert {:ok, ^catalog} = Cache.catalog()
    assert {:ok, ^verses} = Cache.chapter(catalog, "1 Nephi", 1)
  end

  test "rejects mixed-release layers and retries a changing catalog once" do
    {:ok, catalog} = Cache.catalog()
    Fake.reset(:atlas_layer_change)
    assert {:error, :release_changed} = Cache.layers(catalog, [Fixture.key(1)])
    Cache.clear()
    Fake.reset(:atlas_catalog_change)
    assert {:ok, %{release: "fixture-v2"}} = Cache.catalog()
  end

  test "honors truncation and gathers all pages of evidence" do
    {:ok, catalog} = Fixture.catalog()
    Fake.reset(:atlas_paged)
    assert {:ok, layer} = Atlas.layer(catalog, Fixture.key(1))
    assert map_size(layer.coverage) == 2
    assert Enum.any?(Fake.calls(), &String.contains?(&1, "offset 1"))
  end

  test "handles upstream failures, malformed evidence, and invalid reader responses" do
    {:ok, catalog} = Fixture.catalog()
    Fake.reset(:down)
    assert {:error, :unavailable} = Atlas.catalog()
    Fake.reset(:atlas_bad_span)
    assert {:error, :invalid_span} = Atlas.layer(catalog, Fixture.key(1))
    Fake.reset(:atlas_bad_reader)
    assert {:error, :invalid_chapter} = Atlas.chapter(catalog, "1 Nephi", 1)
    assert {:error, :unknown_chapter} = Atlas.chapter(catalog, "1 Nephi", 999)
    assert {:error, :unknown_entry} = Atlas.layer(catalog, "not-an-entry")
  end
end
