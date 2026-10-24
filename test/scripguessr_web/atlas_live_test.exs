defmodule ScripguessrWeb.AtlasLiveTest do
  use Scripguessr.DataCase
  import Phoenix.ConnTest
  import Phoenix.LiveViewTest
  alias Scripguessr.AtlasFixture, as: Fixture
  @endpoint ScripguessrWeb.Endpoint

  setup do
    Scripguessr.Atlas.Cache.clear()
    :ok
  end

  test "Atlas discovers categories and keeps study separate from games" do
    {:ok, view, _} = live(build_conn(), "/atlas")
    html = render_async(view)
    assert html =~ "Book of Mormon Atlas"
    assert has_element?(view, ".atlas-category-toggle", "New category")
    assert has_element?(view, ".atlas-category-toggle", "Places")
    assert has_element?(view, ".atlas-chapter[aria-label='Jacob 1']")
    refute html =~ "Study highlighted"
    refute has_element?(view, "#start-game")
  end

  test "search distinguishes namesakes; selections highlight speech and overlapping chapters" do
    {:ok, view, _} = live(build_conn(), "/atlas")
    render_async(view)
    view |> form("#atlas-search", q: "Nephi") |> render_submit()
    assert has_element?(view, ".atlas-entry", "Founder of the Nephite people.")
    assert has_element?(view, ".atlas-entry", "A later prophet descended from Nephi.")
    view |> element(".atlas-category-toggle[phx-value-category='Person']") |> render_click()
    refute has_element?(view, ".atlas-entry[phx-value-key='#{Fixture.key(1)}']")
    view |> element(".atlas-category-toggle[phx-value-category='Person']") |> render_click()
    view |> element(".atlas-entry[phx-value-key='#{Fixture.key(1)}']") |> render_click()
    render_async(view)
    assert has_element?(view, ".atlas-chapter.highlighted[aria-label='1 Nephi 2']")
    refute has_element?(view, ".atlas-chapter.highlighted[aria-label='Jacob 1']")
    view |> form("#atlas-search", q: "Faith") |> render_change()
    view |> element(".atlas-entry[phx-value-key='#{Fixture.key(3)}']") |> render_click()
    render_async(view)
    assert has_element?(view, ".atlas-chapter[aria-label='1 Nephi 2'] .atlas-overlap-count", "2")
    view |> element(".atlas-chapter[aria-label='1 Nephi 2']") |> render_click()
    render_async(view)
    assert has_element?(view, "[role=dialog]", "1 Nephi 2")
    assert has_element?(view, "#atlas-evidence", "Voice")
    assert has_element?(view, "#atlas-evidence", "Referenced")
    assert has_element?(view, ".atlas-reading-verse.matched[data-verse='2']")
    view |> element("#atlas-reader-close") |> render_click()
    render_async(view)
    refute has_element?(view, "#atlas-reader")
    assert has_element?(view, "#atlas-selected", "Faith")
    view |> element("#atlas-clear") |> render_click()
    render_async(view)
    refute has_element?(view, ".atlas-chapter.highlighted")
  end

  test "deep links restore filters and chapter without depending on numeric corpus IDs" do
    params =
      URI.encode_query(%{"layers" => Fixture.key(1), "book" => "1 Nephi", "chapter" => "2"})

    {:ok, view, _} = live(build_conn(), "/atlas?" <> params)
    render_async(view)
    assert has_element?(view, "#atlas-selected", "Nephi")
    assert has_element?(view, "#atlas-reader-title", "1 Nephi 2")
    assert has_element?(view, ".atlas-reading-verse.matched[data-verse='1']")
    refute has_element?(view, ".atlas-reading-verse.matched[data-verse='3']")
  end

  test "unknown links are harmless and chapter validation prevents arbitrary reader requests" do
    {:ok, view, _} = live(build_conn(), "/atlas?layers=unknown&book=Missing&chapter=999")
    render_async(view)
    assert render(view) =~ "no longer available"
    refute has_element?(view, "#atlas-reader")
    render_click(view, "open-chapter", %{"book" => "1 Nephi' or 1=1", "chapter" => "1"})
    refute has_element?(view, "#atlas-reader")
  end

  test "cold-cache failure offers retry and loaded filters survive an upstream error" do
    Scripguessr.Corpus.Fake.reset(:down)
    {:ok, view, _} = live(build_conn(), "/atlas")
    render_async(view)
    assert has_element?(view, "#atlas-retry")
    Scripguessr.Corpus.Fake.reset()
    view |> element("#atlas-retry") |> render_click()
    render_async(view)
    assert has_element?(view, "#atlas-books")
    Scripguessr.Corpus.Fake.reset(:down)
    render_click(view, "toggle", %{"key" => Fixture.key(1)})
    render_async(view)
    assert has_element?(view, "[role=alert]", "Couldn’t load")
    assert has_element?(view, "#atlas-books")
  end

  test "mobile filters can be opened and closed without changing selection" do
    {:ok, view, _} = live(build_conn(), "/atlas")
    render_async(view)
    view |> element("#atlas-open-filters") |> render_click()
    assert has_element?(view, "#atlas-sidebar.mobile-open")
    view |> element("#atlas-close-filters") |> render_click()
    refute has_element?(view, "#atlas-sidebar.mobile-open")
  end

  test "a stalled highlight request does not block opening or switching chapters" do
    {:ok, view, _} = live(build_conn(), "/atlas")
    render_async(view)
    Scripguessr.Corpus.Fake.block_next("atlas:layer", self())
    render_click(view, "toggle", %{"key" => Fixture.key(1)})
    assert_receive {:corpus_blocked, _, worker}
    on_exit(fn -> Scripguessr.Atlas.Cache.clear() end)

    render_click(view, "open-chapter", %{"book" => "1 Nephi", "chapter" => "1"})
    eventually(fn -> has_element?(view, "#atlas-reader [data-verse='3']") end)
    assert has_element?(view, "#atlas-reader-title", "1 Nephi 1")
    render_click(view, "open-chapter", %{"book" => "1 Nephi", "chapter" => "2"})
    eventually(fn -> has_element?(view, "#atlas-reader [data-verse='3']") end)
    assert has_element?(view, "#atlas-reader-title", "1 Nephi 2")

    send(worker, :release_corpus)
    render_async(view)
    assert has_element?(view, "#atlas-reader-title", "1 Nephi 2")
    assert has_element?(view, "#atlas-reader .matched[data-verse='2']")
  end

  test "late chapter results cannot replace a newly opened chapter or reopen a closed reader" do
    {:ok, view, _} = live(build_conn(), "/atlas")
    render_async(view)
    Scripguessr.Corpus.Fake.block_next("atlas:reader", self())
    render_click(view, "open-chapter", %{"book" => "1 Nephi", "chapter" => "1"})
    assert_receive {:corpus_blocked, _, worker}
    on_exit(fn -> Scripguessr.Corpus.Chapter.clear() end)

    render_click(view, "open-chapter", %{"book" => "1 Nephi", "chapter" => "2"})
    render_async(view)
    assert has_element?(view, "#atlas-reader-title", "1 Nephi 2")
    view |> element("#atlas-reader-close") |> render_click()
    send(worker, :release_corpus)

    # Wait for that shared fetch to complete, even though its original LiveView task was cancelled.
    assert {:ok, _} = Scripguessr.Corpus.Chapter.read("1 Nephi", 1, "fixture-v1")
    render_async(view)
    refute has_element?(view, "#atlas-reader")
  end

  test "chapter text survives highlight changes and highlight failures have a separate error" do
    {:ok, view, _} = live(build_conn(), "/atlas?book=1+Nephi&chapter=1")
    render_async(view)
    assert has_element?(view, "#atlas-reader [data-verse='3']")
    Scripguessr.Corpus.Fake.reset(:atlas_bad_span)
    render_click(view, "toggle", %{"key" => Fixture.key(1)})
    assert has_element?(view, "#atlas-reader [data-verse='3']")
    render_async(view)
    assert has_element?(view, "#atlas-reader [data-verse='3']")
    refute has_element?(view, "#atlas-reader [role=alert]")
    refute render(view) =~ "No recorded matches"
    assert has_element?(view, "[role=alert]", "Couldn’t load")
    refute Enum.any?(Scripguessr.Corpus.Fake.calls(), &String.contains?(&1, "atlas:reader"))
  end

  defp eventually(check, attempts \\ 100)
  defp eventually(check, 0), do: assert(check.())

  defp eventually(check, attempts) do
    unless check.() do
      Process.sleep(10)
      eventually(check, attempts - 1)
    end
  end
end
