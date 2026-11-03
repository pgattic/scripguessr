defmodule Scripguessr.CorpusSQLiteTest do
  use ExUnit.Case, async: false
  alias Scripguessr.Corpus.SQLite
  alias Exqlite.Sqlite3

  setup do
    dir = Path.join(System.tmp_dir!(), "scripguessr-sqlite-#{System.unique_integer([:positive])}")
    File.mkdir_p!(dir)
    on_exit(fn -> File.rm_rf!(dir) end)
    path = Path.join(dir, "scripture.db")
    File.write!(Path.join(dir, "release.txt"), "fixture-release\n")
    {:ok, db} = Sqlite3.open(path)

    :ok =
      Sqlite3.execute(db, """
      create table verse (number integer, text text, extra text);
      insert into verse values (1, 'In the beginning', null);
      """)

    :ok = Sqlite3.close(db)
    start_supervised!({SQLite, database: path})
    %{path: path}
  end

  test "returns the release and string-keyed rows with SQLite value types" do
    assert {:ok,
            %{
              release: "fixture-release",
              truncated: false,
              rows: [%{"number" => 1, "text" => "In the beginning", "extra" => nil}]
            }} = SQLite.query("select * from verse")
  end

  test "does not truncate an Atlas-sized page" do
    assert {:ok, %{truncated: false, rows: rows}} =
             SQLite.query("""
             with recursive numbers(n) as (
               select 1 union all select n+1 from numbers where n<5000
             ) select n from numbers order by n
             """)

    assert length(rows) == 5000
    assert List.last(rows) == %{"n" => 5000}
  end

  test "rejects writes and recovers after invalid SQL" do
    assert {:error, {:sqlite, _}} = SQLite.query("insert into verse values (2, 'changed', null)")
    assert {:error, {:sqlite, _}} = SQLite.query("select * from missing_table")
    assert {:ok, %{rows: [%{"count" => 1}]}} = SQLite.query("select count(*) as count from verse")
  end

  test "serves concurrent readers" do
    results =
      1..16
      |> Task.async_stream(fn _ -> SQLite.query("select number from verse") end,
        max_concurrency: 8
      )
      |> Enum.to_list()

    assert Enum.all?(results, fn
             {:ok, {:ok, %{release: "fixture-release", rows: [%{"number" => 1}]}}} -> true
             _ -> false
           end)
  end

  test "a missing bundle fails before an empty database can be created", %{path: path} do
    missing = path <> ".missing"

    assert_raise RuntimeError, ~r/Corpus database is missing/, fn ->
      SQLite.init(database: missing)
    end

    refute File.exists?(missing)
  end
end
