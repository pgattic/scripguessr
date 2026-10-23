defmodule ScripguessrWeb.AtlasLive do
  use ScripguessrWeb, :live_view
  alias Scripguessr.SavedVerses
  alias ScripguessrWeb.SavedVerseComponents
  import ScripguessrWeb.SavedVerseComponents, only: [save_button: 1]
  alias Scripguessr.Atlas.{Cache, Model}
  @max_layers 8

  @impl true
  def mount(_params, _session, socket) do
    {:ok,
     assign(socket,
       catalog: nil,
       catalog_loading: false,
       catalog_error: nil,
       params: %{},
       selected: [],
       layers: [],
       loading: false,
       error: nil,
       notice: nil,
       search: "",
       expanded: MapSet.new(),
       limits: %{},
       drawer: false,
       chapter: nil,
       reader: nil,
       reader_error: nil,
       saved_verses: SavedVerses.keys(socket.assigns.current_user)
     )}
  end

  @impl true
  def handle_params(params, _uri, socket) do
    socket = assign(socket, :params, params)

    cond do
      socket.assigns.catalog -> {:noreply, apply_params(socket)}
      connected?(socket) and !socket.assigns.catalog_loading -> {:noreply, load_catalog(socket)}
      true -> {:noreply, socket}
    end
  end

  @impl true
  def handle_event("retry-catalog", _, socket), do: {:noreply, load_catalog(socket)}

  def handle_event("reload-corpus", _, socket) do
    Cache.clear()
    {:noreply, socket |> assign(catalog: nil, layers: [], reader: nil) |> load_catalog()}
  end

  def handle_event("retry-selection", _, socket), do: {:noreply, apply_params(socket)}

  def handle_event("search", %{"q" => search}, socket) when is_binary(search) do
    categories = if socket.assigns.catalog, do: socket.assigns.catalog.categories, else: []
    expanded = if String.trim(search) == "", do: MapSet.new(), else: MapSet.new(categories)

    {:noreply,
     assign(socket, search: String.slice(search, 0, 200), limits: %{}, expanded: expanded)}
  end

  def handle_event("category", %{"category" => category}, socket) do
    if socket.assigns.catalog && category in socket.assigns.catalog.categories do
      expanded = socket.assigns.expanded

      expanded =
        if MapSet.member?(expanded, category),
          do: MapSet.delete(expanded, category),
          else: MapSet.put(expanded, category)

      {:noreply, assign(socket, :expanded, expanded)}
    else
      {:noreply, socket}
    end
  end

  def handle_event("more", %{"category" => category}, socket) do
    if socket.assigns.catalog && category in socket.assigns.catalog.categories do
      {:noreply,
       assign(socket, :limits, Map.update(socket.assigns.limits, category, 62, &(&1 + 50)))}
    else
      {:noreply, socket}
    end
  end

  def handle_event("toggle", %{"key" => key}, socket) do
    if socket.assigns.catalog && Map.has_key?(socket.assigns.catalog.by_key, key) do
      keys = socket.assigns.selected

      cond do
        key in keys ->
          {:noreply, navigate(socket, List.delete(keys, key), socket.assigns.chapter)}

        length(keys) >= @max_layers ->
          {:noreply, assign(socket, :notice, "Choose up to #{@max_layers} entries at a time.")}

        true ->
          {:noreply, navigate(socket, keys ++ [key], socket.assigns.chapter)}
      end
    else
      {:noreply, socket}
    end
  end

  def handle_event("clear", _, socket),
    do: {:noreply, navigate(socket, [], socket.assigns.chapter)}

  def handle_event("open-chapter", %{"book" => book, "chapter" => value}, socket) do
    case parse_chapter(socket.assigns.catalog, book, value) do
      nil ->
        {:noreply, socket}

      chapter ->
        {:noreply, socket |> assign(drawer: false) |> navigate(socket.assigns.selected, chapter)}
    end
  end

  def handle_event("save-verse", %{"verse" => value}, socket) when is_binary(value) do
    with {number, ""} when number > 0 <- Integer.parse(value),
         {book, chapter} <- socket.assigns.chapter,
         verses when is_list(verses) <- socket.assigns.reader,
         verse when not is_nil(verse) <- Enum.find(verses, &(&1["verse"] == number)) do
      verse = Map.merge(verse, %{"book" => book, "chapter" => chapter})
      {:noreply, SavedVerseComponents.toggle(socket, verse, socket.assigns.catalog.release)}
    else
      _ -> {:noreply, socket}
    end
  end

  def handle_event("close-chapter", _, socket),
    do: {:noreply, navigate(socket, socket.assigns.selected, nil)}

  def handle_event("open-filters", _, socket), do: {:noreply, assign(socket, :drawer, true)}
  def handle_event("close-filters", _, socket), do: {:noreply, assign(socket, :drawer, false)}
  def handle_event(_, _, socket), do: {:noreply, socket}

  @impl true
  def handle_async(:catalog, {:ok, {:ok, catalog}}, socket) do
    {:noreply,
     socket
     |> assign(catalog: catalog, catalog_loading: false, catalog_error: nil)
     |> apply_params()}
  end

  def handle_async(:catalog, _error, socket) do
    {:noreply,
     assign(socket,
       catalog_loading: false,
       catalog_error: "The Atlas couldn’t load. Please try again."
     )}
  end

  def handle_async(:selection, {:ok, {layers, reader}}, socket) do
    socket =
      case layers do
        {:ok, layers} -> assign(socket, layers: layers, loading: false, error: nil)
        {:error, reason} -> assign(socket, loading: false, error: error_message(reason))
      end

    socket =
      case reader do
        {:ok, verses} -> assign(socket, reader: verses, reader_error: nil)
        {:error, reason} -> assign(socket, reader: nil, reader_error: error_message(reason))
      end

    {:noreply, socket}
  end

  def handle_async(:selection, _error, socket),
    do:
      {:noreply,
       assign(socket,
         loading: false,
         error: "Couldn’t load the highlights. Please try again.",
         reader_error: "Couldn’t load this chapter."
       )}

  defp load_catalog(socket) do
    socket
    |> assign(catalog_loading: true, catalog_error: nil)
    |> start_async(:catalog, fn -> Cache.catalog() end)
  end

  defp apply_params(socket) do
    catalog = socket.assigns.catalog
    params = socket.assigns.params

    requested =
      case params["layers"] do
        value when is_binary(value) ->
          value |> String.slice(0, 4096) |> String.split(",", trim: true) |> Enum.uniq()

        _ ->
          []
      end

    selected =
      requested |> Enum.filter(&Map.has_key?(catalog.by_key, &1)) |> Enum.take(@max_layers)

    chapter = parse_chapter(catalog, params["book"], params["chapter"])

    notice =
      if selected != requested,
        do: "Some selected entries are no longer available. Choose them again from the list."

    socket =
      assign(socket,
        selected: selected,
        chapter: chapter,
        loading: true,
        error: nil,
        notice: notice,
        reader: nil,
        reader_error: nil,
        layers: Enum.filter(socket.assigns.layers, &(&1.entry.key in selected))
      )

    start_async(socket, :selection, fn ->
      layers = Cache.layers(catalog, selected)

      reader =
        case chapter do
          {book, number} -> Cache.chapter(catalog, book, number)
          nil -> {:ok, nil}
        end

      {layers, reader}
    end)
  end

  defp parse_chapter(nil, _, _), do: nil

  defp parse_chapter(catalog, book, value) when is_binary(book) and is_binary(value) do
    case Integer.parse(value) do
      {number, ""} when number > 0 -> if Model.chapter(catalog, book, number), do: {book, number}
      _ -> nil
    end
  end

  defp parse_chapter(_, _, _), do: nil

  defp navigate(socket, keys, chapter) do
    params = if keys == [], do: %{}, else: %{"layers" => Enum.join(keys, ",")}

    params =
      case chapter do
        {book, number} -> Map.merge(params, %{"book" => book, "chapter" => number})
        nil -> params
      end

    path = if map_size(params) == 0, do: "/atlas", else: "/atlas?" <> URI.encode_query(params)
    push_patch(socket, to: path)
  end

  defp error_message(:release_changed),
    do: "The corpus has been updated. Reload the Atlas to continue."

  defp error_message(_), do: "Couldn’t load this content. Please try again."

  # Labels are presentation only; the available categories and hierarchy come from the API.
  defp category_label("Person"), do: "People"
  defp category_label("City"), do: "Cities"
  defp category_label(name), do: name <> "s"
  defp kind_label("mention"), do: "Referenced"
  defp kind_label("speech"), do: "Voice"
  defp kind_label("setting"), do: "Setting"
  defp color(key), do: "hsl(#{:erlang.phash2(key, 360)} 52% 43%)"
  defp swatch(entry), do: "--layer-color: #{color(entry.key)}"

  defp groups(catalog, search) do
    query = String.downcase(String.trim(search))

    catalog.categories
    |> Enum.map(fn category ->
      entries =
        Enum.filter(catalog.entries, fn entry ->
          entry.category == category and
            (query == "" or
               String.contains?(
                 String.downcase(entry.name <> " " <> entry.description <> " " <> entry.type),
                 query
               ))
        end)

      entries =
        Enum.sort_by(entries, fn entry ->
          {search_rank(entry, query), entry.name, entry.description}
        end)

      %{name: category, entries: entries}
    end)
    |> Enum.reject(&(&1.entries == []))
    |> Enum.sort_by(fn group ->
      {Enum.min(Enum.map(group.entries, &search_rank(&1, query))), group.name}
    end)
  end

  defp search_rank(entry, query) do
    name = String.downcase(entry.name)

    cond do
      query == "" -> 0
      name == query -> 0
      String.starts_with?(name, query) -> 1
      true -> 2
    end
  end

  defp coverage(layers),
    do: layers |> Enum.flat_map(&Map.keys(&1.coverage)) |> MapSet.new() |> MapSet.size()

  defp ranges_label(ranges),
    do:
      Enum.map_join(ranges, ", ", fn
        {0, 0} -> "heading"
        {first, last} when first == last -> to_string(first)
        {0, last} -> "heading–#{last}"
        {first, last} -> "#{first}–#{last}"
      end)

  @impl true
  def render(assigns) do
    assigns =
      assign(assigns,
        groups: if(assigns.catalog, do: groups(assigns.catalog, assigns.search), else: []),
        chapter_hits:
          if(assigns.chapter, do: Model.hits(assigns.layers, assigns.chapter), else: [])
      )

    atlas(assigns)
  end

  embed_templates "atlas.html"
end
