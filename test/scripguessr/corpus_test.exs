defmodule Scripguessr.CorpusTest do
  use ExUnit.Case, async: false
  alias Scripguessr.Corpus
  alias Corpus.{Cache, Fake}

  setup do
    Fake.reset()
    Cache.clear()
    :ok
  end

  test "prepares five distinct nonempty prompts and reuses metadata" do
    assert {:ok, snapshot} = Cache.prepare()
    assert length(snapshot["rounds"]) == 5
    assert length(Enum.uniq(snapshot["rounds"])) == 5
    assert Enum.all?(snapshot["rounds"], &(String.trim(&1["text"]) != ""))
    calls = length(Fake.calls())
    assert {:ok, _} = Cache.prepare()
    assert length(Fake.calls()) == calls + 1
  end

  test "paginates the pool instead of silently keeping just the first page" do
    Fake.reset(:paginated)
    assert {:ok, catalog} = Corpus.catalog()
    assert length(catalog.references) == 1001
    assert Enum.any?(Fake.calls(), &String.contains?(&1, "offset 1000"))
  end

  test "refreshes metadata once when a release changes during preparation" do
    Fake.reset(:release_once)
    assert {:ok, snapshot} = Cache.prepare()
    assert snapshot["release"] == "fixture-v2"
  end

  test "does not retry changing releases indefinitely or return partial prompts" do
    Fake.reset(:release_always)
    assert {:error, :release_changed} = Cache.prepare()
    Fake.reset(:missing_text)
    assert {:error, :invalid_text} = Cache.prepare()
  end

  test "returns an upstream failure without a prepared game" do
    Fake.reset(:down)
    assert {:error, :unavailable} = Cache.prepare()
  end

  test "concurrent games share metadata but each fetches its own verse selection" do
    games = for _ <- 1..6, do: Task.async(fn -> Cache.prepare() end)

    snapshots =
      Enum.map(games, fn task ->
        {:ok, snapshot} = Task.await(task)
        snapshot
      end)

    assert length(snapshots) == 6
    assert Enum.count(Fake.calls(), &String.contains?(&1, "group_concat")) == 6
    assert Enum.count(Fake.calls(), &String.contains?(&1, "exists")) == 1
  end
end
