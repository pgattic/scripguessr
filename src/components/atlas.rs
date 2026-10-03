use dioxus::prelude::*;

use super::actions::{create_study_set, request_study_game};
use super::ui::{Callout, ChapterReader, HomeButton, Readout, use_game};
use crate::api::ChapterVerse;
use crate::atlas::{
    AtlasCategory, LAYERS, book_chronology, chronology_title, layers_covering, practice_set,
};
use crate::game::catalog;
use crate::loader::load_chapter;
use crate::scriptures::{Canon, ChapterRef};

#[component]
pub fn AtlasPanel(load_error: Option<String>) -> Element {
    let game = use_game();
    let navigator = use_navigator();
    let (books, loading_game, signed_in) = {
        let game = game.read();
        (
            catalog::books(&game.study_catalog, Canon::BookOfMormon).to_vec(),
            game.loading_game,
            game.account.user.is_some(),
        )
    };
    let mut selected_layers = use_signal(Vec::<&'static str>::new);
    let mut collapsed_categories = use_signal(|| AtlasCategory::ALL.to_vec());
    let mut layer_filters = use_signal(|| vec![String::new(); AtlasCategory::ALL.len()]);
    let mut mobile_layers_open = use_signal(|| false);
    let mut selected_chapter = use_signal(|| None::<ChapterRef>);
    let mut reader = use_signal(|| None::<(ChapterRef, Vec<ChapterVerse>)>);
    let mut saved_atlas_selection = use_signal(|| None::<Vec<&'static str>>);
    let reader_loading = use_signal(|| false);
    let reader_error = use_signal(|| None::<String>);
    let active_ids = selected_layers();
    let selection_saved = saved_atlas_selection().as_ref() == Some(&active_ids);
    let practice_set = practice_set(&active_ids, &books);
    let selected = selected_chapter();

    rsx! {
        div { class: "atlas-layout",
            section { class: "panel atlas-map-panel",
                div { class: "picker-header atlas-heading",
                    div {
                        h2 { "Book of Mormon Atlas" }
                        span { class: "muted", "The record at chapter scale" }
                    }
                    div { class: "actions atlas-top-actions",
                        HomeButton {}
                        if let Some(set) = practice_set.clone() {
                            {
                                let practice_set = set.clone();
                                let saved_set = set;
                                let saved_selection = active_ids.clone();
                                rsx! {
                                    button {
                                        class: "button",
                                        disabled: loading_game,
                                        onclick: move |_| request_study_game(
                                            game,
                                            navigator,
                                            practice_set.clone(),
                                            10.min(practice_set.passages.len()),
                                        ),
                                        if loading_game { "Starting" } else { "Practice highlighted" }
                                    }
                                    button {
                                        class: "button secondary",
                                        disabled: selection_saved || !signed_in,
                                        onclick: move |_| {
                                            let selection = saved_selection.clone();
                                            create_study_set(game, saved_set.clone(), move |_| {
                                                saved_atlas_selection.set(Some(selection));
                                            });
                                        },
                                        if !signed_in { "Sign in to save" } else if selection_saved { "Saved" } else { "Save as study set" }
                                    }
                                }
                            }
                        }
                    }
                }

                div { class: "atlas-mobile-layer-bar",
                    button {
                        class: "button secondary compact-button",
                        onclick: move |_| mobile_layers_open.set(true),
                        "Layers"
                    }
                    span { class: "muted",
                        if active_ids.len() == 1 {
                            "1 selected"
                        } else {
                            "{active_ids.len()} selected"
                        }
                    }
                }

                if books.is_empty() {
                    if let Some(error) = load_error {
                        Callout { title: "Atlas unavailable", message: error }
                    } else {
                        Readout { label: "Scripture catalog", class: "atlas-loading",
                            strong { "Loading" }
                        }
                    }
                } else {
                    div { class: "atlas-scroll",
                        div { class: "atlas-books",
                            for book in books.iter() {
                                {
                                    let chronology = book_chronology(&book.name);
                                    let chronology_title = chronology_title(&book.name);
                                    rsx! {
                                    div { class: "atlas-book-row",
                                    div { class: "atlas-book-heading",
                                        strong { class: "atlas-book-name", "{book.name}" }
                                        if let Some(chronology) = chronology {
                                            span {
                                                class: "atlas-book-dates",
                                                title: chronology_title,
                                                "{chronology.dates}"
                                            }
                                        }
                                    }
                                    div { class: "atlas-chapters chapter-matrix",
                                        for chapter in book.chapters.iter().copied() {
                                            {
                                                let chapter_ref = ChapterRef {
                                                    canon: Canon::BookOfMormon,
                                                    book: book.name.clone(),
                                                    chapter,
                                                };
                                                let hits = layers_covering(&active_ids, &book.name, chapter);
                                                let title = if hits.is_empty() {
                                                    chapter_ref.to_string()
                                                } else {
                                                    format!(
                                                        "{chapter_ref} · {}",
                                                        hits.iter().map(|layer| layer.name).collect::<Vec<_>>().join(" + ")
                                                    )
                                                };
                                                rsx! {
                                                    button {
                                                        class: "atlas-chapter",
                                                        title,
                                                        aria_label: "{book.name} chapter {chapter}",
                                                        onclick: move |_| {
                                                            if selected_chapter().as_ref() == Some(&chapter_ref) {
                                                                selected_chapter.set(None);
                                                            } else {
                                                                selected_chapter.set(Some(chapter_ref.clone()));
                                                            }
                                                        },
                                                        span { "{chapter}" }
                                                        if !hits.is_empty() {
                                                            i { class: "atlas-hit-bands", aria_hidden: "true",
                                                                for hit in hits.iter() {
                                                                    b { class: "tone-{hit.tone}" }
                                                                }
                                                            }
                                                        }
                                                        if hits.len() > 1 {
                                                            small {
                                                                class: "atlas-overlap-count",
                                                                aria_label: "{hits.len()} active layers overlap",
                                                                "{hits.len()}"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    } }
                                }
                            }
                        }
                    }
                    p { class: "atlas-date-note muted",
                        "Dates are approximate and follow the chronology presented in the Book of Mormon. Ether appears in record order, although its history is earlier."
                    }
                }
            }

            aside { class: if mobile_layers_open() { "atlas-sidebar mobile-open" } else { "atlas-sidebar" },
                section { class: "panel atlas-layers-panel",
                    div { class: "picker-header",
                        h2 { "Layers" }
                        div { class: "atlas-layer-actions",
                            button {
                                class: "text-button",
                                disabled: active_ids.len() == LAYERS.len(),
                                onclick: move |_| selected_layers.set(
                                    LAYERS.iter().map(|layer| layer.id).collect()
                                ),
                                "Select all"
                            }
                            button {
                                class: "text-button",
                                disabled: active_ids.is_empty(),
                                onclick: move |_| selected_layers.set(Vec::new()),
                                "Clear"
                            }
                            button {
                                class: "button secondary compact-button atlas-mobile-layer-close",
                                onclick: move |_| mobile_layers_open.set(false),
                                "Done"
                            }
                        }
                    }
                    for category in AtlasCategory::ALL {
                        div { class: "atlas-layer-group",
                            {
                                let collapsed = collapsed_categories().contains(&category);
                                let category_layers = LAYERS
                                    .iter()
                                    .copied()
                                    .filter(|item| item.category == category)
                                    .collect::<Vec<_>>();
                                let selected_count = category_layers
                                    .iter()
                                    .filter(|item| active_ids.contains(&item.id))
                                    .count();
                                rsx! {
                                    button {
                                        class: "atlas-category-toggle",
                                        aria_expanded: !collapsed,
                                        onclick: move |_| {
                                            let mut next = collapsed_categories();
                                            if collapsed {
                                                next.retain(|candidate| candidate != &category);
                                            } else {
                                                next.push(category);
                                            }
                                            collapsed_categories.set(next);
                                        },
                                        span { class: "setup-label",
                                            "{category.label()}"
                                            small { "{selected_count}/{category_layers.len()}" }
                                        }
                                        span { class: "atlas-category-symbol", aria_hidden: "true",
                                            if collapsed { "+" } else { "-" }
                                        }
                                    }
                                    if !collapsed {
                                        div { class: "atlas-category-layers",
                                            div { class: "atlas-category-tools",
                                                input {
                                                    class: "atlas-layer-filter",
                                                    r#type: "search",
                                                    aria_label: "Filter {category.label()}",
                                                    placeholder: "Filter {category.label().to_lowercase()}",
                                                    value: layer_filters()[category.index()].clone(),
                                                    oninput: move |event| {
                                                        let mut next = layer_filters();
                                                        next[category.index()] = event.value();
                                                        layer_filters.set(next);
                                                    }
                                                }
                                                div { class: "atlas-category-actions",
                                                    button {
                                                        class: "text-button",
                                                        disabled: selected_count == category_layers.len(),
                                                        onclick: move |_| {
                                                            let mut next = selected_layers();
                                                            for item in LAYERS.iter().filter(|item| item.category == category) {
                                                                if !next.contains(&item.id) {
                                                                    next.push(item.id);
                                                                }
                                                            }
                                                            selected_layers.set(next);
                                                        },
                                                        "Select all"
                                                    }
                                                    button {
                                                        class: "text-button",
                                                        disabled: selected_count == 0,
                                                        onclick: move |_| {
                                                            let mut next = selected_layers();
                                                            next.retain(|id| {
                                                                !LAYERS.iter().any(|item| {
                                                                    item.category == category && item.id == *id
                                                                })
                                                            });
                                                            selected_layers.set(next);
                                                        },
                                                        "Clear"
                                                    }
                                                }
                                            }
                                            div { class: "atlas-category-scroll",
                                            {
                                                let filter = layer_filters()[category.index()].trim().to_lowercase();
                                                let visible_layers = category_layers
                                                    .iter()
                                                    .copied()
                                                    .filter(|item| filter.is_empty()
                                                        || item.name.to_lowercase().contains(&filter)
                                                        || item.summary.to_lowercase().contains(&filter))
                                                    .collect::<Vec<_>>();
                                                rsx! {
                                            for atlas_item in visible_layers.iter().copied() {
                                                {
                                                    let active = active_ids.contains(&atlas_item.id);
                                                    let id = atlas_item.id;
                                                    rsx! {
                                                        label { class: if active { "atlas-layer-toggle active" } else { "atlas-layer-toggle" },
                                                            input {
                                                                r#type: "checkbox",
                                                                checked: active,
                                                                onchange: move |_| {
                                                                    let mut next = selected_layers();
                                                                    if active {
                                                                        next.retain(|candidate| *candidate != id);
                                                                    } else {
                                                                        next.push(id);
                                                                    }
                                                                    selected_layers.set(next);
                                                                }
                                                            }
                                                            i { class: "atlas-layer-swatch tone-{atlas_item.tone}" }
                                                            strong { "{atlas_item.name}" }
                                                        }
                                                    }
                                                }
                                            }
                                            if visible_layers.is_empty() {
                                                span { class: "atlas-filter-empty muted", "No matching layers" }
                                            }
                                                }
                                            }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

            }
        }

        if mobile_layers_open() {
            div {
                class: "atlas-mobile-layer-backdrop",
                onclick: move |_| mobile_layers_open.set(false),
            }
        }

        if let Some(chapter) = selected.clone() {
            div {
                class: "dialog-backdrop",
                onclick: move |_| selected_chapter.set(None),
                div {
                    class: "atlas-chapter-overlay",
                    role: "dialog",
                    aria_modal: "true",
                    onclick: move |event| event.stop_propagation(),
                    div { class: "dialog-header",
                        h2 { "{chapter}" }
                        button {
                            class: "button secondary compact-button",
                            onclick: move |_| selected_chapter.set(None),
                            "Close"
                        }
                    }
                    div { class: "atlas-overlay-content",
                        div { class: "atlas-chapter-layers",
                            for atlas_item in LAYERS.iter().copied().filter(|item| item.span_for(&chapter.book, chapter.chapter).is_some()) {
                                div { class: "atlas-overlay-layer",
                                    i { class: "atlas-layer-swatch tone-{atlas_item.tone}" }
                                    div {
                                        strong { "{atlas_item.name}" }
                                        span { "{atlas_item.summary}" }
                                    }
                                }
                            }
                            if !LAYERS.iter().any(|item| item.span_for(&chapter.book, chapter.chapter).is_some()) {
                                span { class: "muted", "No curated layers include this chapter." }
                            }
                        }
                        if let Some(error) = reader_error() {
                            Callout { title: "Chapter unavailable", message: error }
                        }
                        div { class: "actions atlas-overlay-actions",
                            button {
                                class: "button",
                                disabled: reader_loading(),
                                onclick: move |_| request_atlas_chapter(
                                    reader,
                                    reader_loading,
                                    reader_error,
                                    selected_chapter,
                                    chapter.clone(),
                                ),
                                if reader_loading() { "Loading" } else { "Read chapter" }
                            }
                        }
                    }
                }
            }
        }

        if let Some((chapter, verses)) = reader() {
            ChapterReader {
                title: chapter.to_string(),
                verses,
                on_close: move |_| reader.set(None),
            }
        }
    }
}

fn request_atlas_chapter(
    mut reader: Signal<Option<(ChapterRef, Vec<ChapterVerse>)>>,
    mut loading: Signal<bool>,
    mut error: Signal<Option<String>>,
    mut selected_chapter: Signal<Option<ChapterRef>>,
    chapter: ChapterRef,
) {
    loading.set(true);
    error.set(None);
    spawn(async move {
        match load_chapter(chapter.clone()).await {
            Ok(response) => {
                selected_chapter.set(None);
                reader.set(Some((chapter, response.verses)));
            }
            Err(message) => error.set(Some(message)),
        }
        loading.set(false);
    });
}
