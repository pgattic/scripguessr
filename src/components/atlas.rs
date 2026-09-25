use dioxus::prelude::*;

use super::{ChapterReader, request_study_game};
use crate::api::{ChapterRequest, ChapterVerse};
use crate::atlas::{
    AtlasCategory, AtlasLayer, LAYERS, book_chronology, era_starting_at, layer as atlas_layer,
};
use crate::game::Game;
use crate::loader::load_chapter;
use crate::routes::Route;
use crate::scriptures::{BookInfo, Canon};
use crate::study_sets::{PromptPolicy, StudyGuessScope, StudyPassage, StudySet};

#[derive(Clone, PartialEq)]
struct AtlasReaderData {
    title: String,
    answer: StudyPassage,
    verses: Vec<ChapterVerse>,
}

#[component]
pub(super) fn AtlasPanel(game: Signal<Game>, load_error: Option<String>) -> Element {
    let navigator = use_navigator();
    use_effect(clear_deprecated_atlas_storage);
    let snapshot = game.read().clone();
    let books = snapshot
        .study_metadata
        .iter()
        .find(|metadata| metadata.canon == Canon::BookOfMormon)
        .map(|metadata| metadata.books.clone())
        .unwrap_or_default();
    let mut selected_layers = use_signal(Vec::<String>::new);
    let mut collapsed_categories = use_signal(|| AtlasCategory::ALL.to_vec());
    let mut mobile_layers_open = use_signal(|| false);
    let mut selected_chapter = use_signal(|| None::<(String, u16)>);
    let mut reader = use_signal(|| None::<AtlasReaderData>);
    let mut saved_atlas_selection = use_signal(|| None::<Vec<String>>);
    let reader_loading = use_signal(|| false);
    let reader_error = use_signal(|| None::<String>);
    let active_ids = selected_layers();
    let selection_saved = saved_atlas_selection().as_ref() == Some(&active_ids);
    let practice_set = atlas_practice_set(&active_ids, &books);
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
                        button {
                            class: "button secondary",
                            onclick: move |_| {
                                game.write().change_settings();
                                navigator.push(Route::Setup {});
                            },
                            "Home"
                        }
                        if let Some(set) = practice_set.clone() {
                            {
                                let practice_set = set.clone();
                                let saved_set = set;
                                let saved_selection = active_ids.clone();
                                rsx! {
                                    button {
                                        class: "button",
                                        disabled: snapshot.loading_game,
                                        onclick: move |_| request_study_game(
                                            game,
                                            navigator,
                                            practice_set.clone(),
                                            10.min(practice_set.passages.len()),
                                        ),
                                        if snapshot.loading_game { "Starting" } else { "Practice highlighted" }
                                    }
                                    button {
                                        class: "button secondary",
                                        disabled: selection_saved,
                                        onclick: move |_| {
                                            game.write().save_custom_study_set(saved_set.clone());
                                            saved_atlas_selection.set(Some(saved_selection.clone()));
                                        },
                                        if selection_saved { "Saved" } else { "Save as study set" }
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
                        div { class: "callout warning",
                            strong { "Atlas unavailable" }
                            span { "{error}" }
                        }
                    } else {
                        div { class: "ready atlas-loading",
                            span { class: "muted", "Scripture catalog" }
                            strong { "Loading" }
                        }
                    }
                } else {
                    div { class: "atlas-scroll",
                        div { class: "atlas-books",
                            for book in books.iter() {
                                {
                                    let chronology = book_chronology(&book.name);
                                    let era = era_starting_at(&book.name);
                                    let chronology_title = chronology.map(|chronology| {
                                        if let Some(era) = era {
                                            format!(
                                                "{}. {}. Dates are approximate.",
                                                chronology.note, era.name
                                            )
                                        } else {
                                            format!("{}. Dates are approximate.", chronology.note)
                                        }
                                    });
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
                                                let book_name = book.name.clone();
                                                let hits = atlas_hits(&active_ids, &book.name, chapter);
                                                let title = if hits.is_empty() {
                                                    format!("{} {}", book.name, chapter)
                                                } else {
                                                    format!(
                                                        "{} {} · {}",
                                                        book.name,
                                                        chapter,
                                                        hits.iter().map(|layer| layer.name).collect::<Vec<_>>().join(" + ")
                                                    )
                                                };
                                                rsx! {
                                                    button {
                                                        class: "atlas-chapter",
                                                        title,
                                                        aria_label: "{book.name} chapter {chapter}",
                                                        onclick: move |_| {
                                                            let next = (book_name.clone(), chapter);
                                                            if selected_chapter().as_ref() == Some(&next) {
                                                                selected_chapter.set(None);
                                                            } else {
                                                                selected_chapter.set(Some(next));
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
                                    LAYERS.iter().map(|layer| layer.id.to_string()).collect()
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
                                        span { class: "setup-label", "{category.label()}" }
                                        span { class: "atlas-category-symbol", aria_hidden: "true",
                                            if collapsed { "+" } else { "-" }
                                        }
                                    }
                                    if !collapsed {
                                        div { class: "atlas-category-layers",
                                            for atlas_item in LAYERS.iter().copied().filter(|item| item.category == category) {
                                                {
                                                    let active = active_ids.iter().any(|id| id == atlas_item.id);
                                                    let id = atlas_item.id.to_string();
                                                    rsx! {
                                                        label { class: if active { "atlas-layer-toggle active" } else { "atlas-layer-toggle" },
                                                            input {
                                                                r#type: "checkbox",
                                                                checked: active,
                                                                onchange: move |_| {
                                                                    let mut next = selected_layers();
                                                                    if active {
                                                                        next.retain(|candidate| candidate != &id);
                                                                    } else {
                                                                        next.push(id.clone());
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

        if let Some((book, chapter)) = selected.clone() {
            div {
                class: "dialog-backdrop",
                onclick: move |_| selected_chapter.set(None),
                div {
                    class: "atlas-chapter-overlay",
                    role: "dialog",
                    aria_modal: "true",
                    onclick: move |event| event.stop_propagation(),
                    div { class: "dialog-header",
                        h2 { "{book} {chapter}" }
                        button {
                            class: "button secondary compact-button",
                            onclick: move |_| selected_chapter.set(None),
                            "Close"
                        }
                    }
                    div { class: "atlas-overlay-content",
                        div { class: "atlas-chapter-layers",
                            for atlas_item in LAYERS.iter().copied().filter(|item| item.span_for(&book, chapter).is_some()) {
                                div { class: "atlas-overlay-layer",
                                    i { class: "atlas-layer-swatch tone-{atlas_item.tone}" }
                                    div {
                                        strong { "{atlas_item.name}" }
                                        span { "{atlas_item.summary}" }
                                    }
                                }
                            }
                            if !LAYERS.iter().any(|item| item.span_for(&book, chapter).is_some()) {
                                span { class: "muted", "No curated layers include this chapter." }
                            }
                        }
                        if let Some(error) = reader_error() {
                            div { class: "callout warning",
                                strong { "Chapter unavailable" }
                                span { "{error}" }
                            }
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
                                    book.clone(),
                                    chapter,
                                ),
                                if reader_loading() { "Loading" } else { "Read chapter" }
                            }
                        }
                    }
                }
            }
        }

        if let Some(data) = reader() {
            ChapterReader {
                title: data.title,
                answer: data.answer,
                verses: data.verses,
                on_close: move |_| reader.set(None),
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn clear_deprecated_atlas_storage() {
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        let _ = storage.remove_item("scripguessr.atlas.v1");
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn clear_deprecated_atlas_storage() {}

fn atlas_hits(selected: &[String], book: &str, chapter: u16) -> Vec<AtlasLayer> {
    selected
        .iter()
        .filter_map(|id| atlas_layer(id))
        .filter(|layer| layer.span_for(book, chapter).is_some())
        .collect()
}

fn atlas_practice_set(selected: &[String], books: &[BookInfo]) -> Option<StudySet> {
    let layers = selected
        .iter()
        .filter_map(|id| atlas_layer(id))
        .collect::<Vec<_>>();
    if layers.is_empty() {
        return None;
    }
    let passages = books
        .iter()
        .flat_map(|book| {
            let layers = &layers;
            book.chapters.iter().filter_map(move |chapter| {
                if !layers
                    .iter()
                    .any(|layer| layer.span_for(&book.name, *chapter).is_some())
                {
                    return None;
                }
                let verse_count = book.verse_count(*chapter)?;
                Some(StudyPassage {
                    canon: Canon::BookOfMormon,
                    book: book.name.clone(),
                    chapter: *chapter,
                    verses: (1..=verse_count).collect(),
                })
            })
        })
        .collect::<Vec<_>>();
    if passages.is_empty() {
        return None;
    }
    let name = if layers.len() == 1 {
        layers[0].name.to_string()
    } else {
        "Atlas selection".to_string()
    };
    Some(StudySet {
        id: "atlas-selection".to_string(),
        name,
        passages,
        guess_scope: StudyGuessScope::FullCanons,
        prompt_policy: PromptPolicy::SingleVerse,
    })
}

fn request_atlas_chapter(
    mut reader: Signal<Option<AtlasReaderData>>,
    mut loading: Signal<bool>,
    mut error: Signal<Option<String>>,
    mut selected_chapter: Signal<Option<(String, u16)>>,
    book: String,
    chapter: u16,
) {
    loading.set(true);
    error.set(None);
    spawn(async move {
        match load_chapter(ChapterRequest {
            canon: Canon::BookOfMormon,
            book: book.clone(),
            chapter,
        })
        .await
        {
            Ok(response) => {
                selected_chapter.set(None);
                reader.set(Some(AtlasReaderData {
                    title: format!("{book} {chapter}"),
                    answer: StudyPassage {
                        canon: Canon::BookOfMormon,
                        book,
                        chapter,
                        verses: Vec::new(),
                    },
                    verses: response.verses,
                }));
                loading.set(false);
            }
            Err(message) => {
                error.set(Some(message));
                loading.set(false);
            }
        }
    });
}
