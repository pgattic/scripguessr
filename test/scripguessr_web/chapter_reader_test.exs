defmodule ScripguessrWeb.ChapterReaderTest do
  use ExUnit.Case, async: true
  import Phoenix.LiveViewTest
  alias ScripguessrWeb.ChapterReader

  defp reader(marks, opts \\ []) do
    defaults = [
      id: "reader",
      chapter: {"1 Nephi", 1},
      verses: for(number <- 1..5, do: %{"verse" => number, "text" => "Passage #{number}."}),
      user: nil,
      saved: MapSet.new(),
      marks: marks
    ]

    render_component(&ChapterReader.chapter_reader/1, Keyword.merge(defaults, opts))
    |> LazyHTML.from_fragment()
  end

  defp has?(html, selector), do: Enum.any?(LazyHTML.query(html, selector))
  defp mark(key), do: %{key: key, label: key, color: "green"}

  test "contiguous highlights join while disjoint verses retain their outer corners" do
    a = mark("a")
    html = reader(%{1 => [a], 2 => [a], 4 => [a]})
    assert has?(html, "[data-verse='1'].highlight-joins-after")
    refute has?(html, "[data-verse='1'].highlight-joins-before")
    assert has?(html, "[data-verse='2'].highlight-joins-before")
    refute has?(html, "[data-verse='2'].highlight-joins-after")
    refute has?(html, "[data-verse='3'].matched")
    refute has?(html, "[data-verse='4'].highlight-joins-before")
    refute has?(html, "[data-verse='4'].highlight-joins-after")
    assert has?(html, "[data-verse='1'] [data-layer='a'].active.band-joins-after")
    assert has?(html, "[data-verse='2'] [data-layer='a'].active.band-joins-before")
    refute has?(html, "[data-verse='4'] [data-layer='a'].band-joins-before")
  end

  test "overlapping entries occupy stable lanes and join only their own contiguous range" do
    a = mark("a")
    b = mark("b")
    html = reader(%{1 => [a], 2 => [b, a], 3 => [b]})

    for number <- 1..3 do
      lanes =
        html
        |> LazyHTML.query("[data-verse='#{number}'] [data-layer]")
        |> LazyHTML.attribute("data-layer")

      assert lanes == ["a", "b"]
    end

    assert has?(html, "[data-verse='2'].highlight-joins-before.highlight-joins-after")
    assert has?(html, "[data-verse='1'] [data-layer='a'].band-joins-after")
    refute has?(html, "[data-verse='1'] [data-layer='b'].active")
    assert has?(html, "[data-verse='2'] [data-layer='a'].band-joins-before")
    refute has?(html, "[data-verse='2'] [data-layer='a'].band-joins-after")
    assert has?(html, "[data-verse='2'] [data-layer='b'].band-joins-after")
    refute has?(html, "[data-verse='2'] [data-layer='b'].band-joins-before")
    assert has?(html, "[data-verse='3'] [data-layer='b'].band-joins-before")
  end

  test "chapter headings and a single game target do not join into verse runs" do
    html =
      reader(%{0 => [mark("a")]},
        target: 1,
        verses: [%{"verse" => 0, "text" => "Heading."}, %{"verse" => 1, "text" => "Verse."}]
      )

    assert has?(html, ".atlas-reading-heading.matched")
    assert has?(html, "[data-verse='1'].matched[aria-current=true]")
    refute has?(html, "[data-verse='1'].highlight-joins-before")
    refute has?(html, "[data-verse='1'].highlight-joins-after")
  end
end
