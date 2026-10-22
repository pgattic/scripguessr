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

  test "Atlas discovers categories and keeps study separate from quizzes" do
    {:ok, view, _} = live(build_conn(), "/atlas")
    html = render_async(view)
    assert html =~ "Book of Mormon Atlas"
    assert has_element?(view, ".atlas-category-toggle", "New category")
    assert has_element?(view, ".atlas-category-toggle", "Places")
    assert has_element?(view, ".atlas-chapter[aria-label='Jacob 1']")
    refute html =~ "Study highlighted"
    refute has_element?(view, "#start-quiz")
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
end
