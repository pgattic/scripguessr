use dioxus::prelude::*;

use super::{StatsPanel, request_new_game};
use crate::game::Game;
use crate::routes::Route;
use crate::scriptures::{BookScope, Canon, Difficulty, GameMode};

#[component]
pub(super) fn SetupPanel(game: Signal<Game>, load_error: Option<String>) -> Element {
    let navigator = use_navigator();
    let mut scope_editor_open = use_signal(|| false);
    let snapshot = game.read().clone();
    let metadata_current = snapshot.metadata_current();
    let metadata_ready = snapshot.metadata_ready();

    rsx! {
        div { class: "setup-layout",
            section { class: "panel setup-panel",
                div { class: "picker-header",
                    h2 { "New game" }
                    span { class: "muted", "{snapshot.settings.selection_label()}" }
                }

                div { class: "scope-summary",
                    span { class: "muted", "Scope" }
                    strong { "{snapshot.settings.selection_label()}" }
                    if metadata_current {
                        span { class: "muted", "{snapshot.verse_count_for_difficulty()} of {snapshot.total_verse_count()} verses in play" }
                    } else {
                        span { class: "muted", "Updating verse pool" }
                    }
                }

                div { class: "setup-group",
                    span { class: "setup-label", "Presets" }
                    div { class: "preset-row",
                        for mode in GameMode::ALL {
                            button {
                                class: if mode.scope() == snapshot.settings.scope { "segment active" } else { "segment" },
                                onclick: move |_| game.write().set_preset(mode),
                                "{mode.label()}"
                            }
                        }
                    }
                }

                div { class: "actions scope-actions",
                    button {
                        class: "button secondary",
                        onclick: move |_| scope_editor_open.toggle(),
                        if scope_editor_open() { "Close scope editor" } else { "Customize scope" }
                    }
                    button {
                        class: "button secondary",
                        onclick: move |_| {
                            navigator.push(Route::StudyIndex {});
                        },
                        "Study sets"
                    }
                    button {
                        class: "button secondary",
                        onclick: move |_| {
                            navigator.push(Route::Atlas {});
                        },
                        "Atlas"
                    }
                }

                if scope_editor_open() {
                    ScopeEditor { game, snapshot: snapshot.clone() }
                }

                div { class: "setup-group",
                    span { class: "setup-label", "Rounds" }
                    div { class: "segmented",
                        for round_count in [5_usize, 10] {
                            button {
                                class: if snapshot.settings.round_count == round_count { "segment active" } else { "segment" },
                                onclick: move |_| game.write().set_round_count(round_count),
                                "{round_count}"
                            }
                        }
                    }
                }

                div { class: "setup-group",
                    span { class: "setup-label", "Difficulty" }
                    div { class: "segmented",
                        for difficulty in Difficulty::ALL {
                            button {
                                class: if snapshot.settings.difficulty == difficulty { "segment active" } else { "segment" },
                                onclick: move |_| game.write().set_difficulty(difficulty),
                                "{difficulty.label()}"
                            }
                        }
                    }
                }

                div { class: "actions",
                    button {
                        class: "button",
                        disabled: !metadata_ready || snapshot.loading_game,
                        onclick: move |_| request_new_game(game, navigator),
                        if snapshot.loading_game { "Starting" } else { "Start" }
                    }
                }
                if let Some(error) = snapshot.error.as_ref() {
                    div { class: "callout warning",
                        strong { "Game unavailable" }
                        span { "{error}" }
                    }
                } else if metadata_current && !metadata_ready {
                    div { class: "callout warning",
                        strong { "Scope unavailable" }
                        span { "Choose at least one book with playable verses." }
                    }
                }
            }

            aside { class: "panel setup-side",
                div { class: "ready",
                    span { class: "muted", "Verse pool" }
                    if metadata_ready {
                        strong { "{snapshot.verse_count_for_difficulty()} of {snapshot.total_verse_count()} verses" }
                    } else if metadata_current {
                        strong { "No playable verses" }
                    } else if load_error.is_some() {
                        strong { "Could not load game data" }
                    } else {
                        strong { "Loading game data" }
                    }
                }

                if let Some(error) = load_error {
                    div { class: "callout warning",
                        strong { "Game server unavailable" }
                        span { "{error}" }
                    }
                }

                StatsPanel { game, stats: snapshot.stats.clone() }
            }
        }
    }
}

#[component]
fn ScopeEditor(game: Signal<Game>, snapshot: Game) -> Element {
    rsx! {
        div { class: "scope-editor",
            for canon in Canon::ALL {
                ScopeCanonRow {
                    game,
                    snapshot: snapshot.clone(),
                    canon,
                }
            }
        }
    }
}

#[component]
fn ScopeCanonRow(game: Signal<Game>, snapshot: Game, canon: Canon) -> Element {
    let mut book_filter = use_signal(String::new);
    let selected = snapshot.settings.scope.contains_canon(canon);
    let canon_scope = snapshot.settings.scope.canon_scope(canon).cloned();
    let books = snapshot.all_books_for(canon);
    let all_book_names = books
        .iter()
        .map(|book| book.name.clone())
        .collect::<Vec<_>>();
    let book_scope = canon_scope
        .as_ref()
        .map(|scope| scope.books.clone())
        .unwrap_or(BookScope::All);
    let choosing_books = matches!(book_scope, BookScope::Selected(_));
    let selected_books = match &book_scope {
        BookScope::All => Vec::new(),
        BookScope::Selected(books) => books.clone(),
    };
    let filter_value = book_filter();
    let normalized_filter = filter_value.trim().to_lowercase();
    let filtered_books = books
        .iter()
        .filter(|book| {
            normalized_filter.is_empty() || book.name.to_lowercase().contains(&normalized_filter)
        })
        .cloned()
        .collect::<Vec<_>>();
    let summary = if !selected {
        "Off".to_string()
    } else {
        match &book_scope {
            BookScope::All => "All books".to_string(),
            BookScope::Selected(books) if books.len() == 1 => format!("{} only", books[0]),
            BookScope::Selected(books) => format!("{} books", books.len()),
        }
    };
    let select_all_books = all_book_names.clone();

    rsx! {
        div { class: if selected { "scope-canon selected" } else { "scope-canon" },
            div { class: "scope-canon-header",
                button {
                    class: if selected { "choice active" } else { "choice" },
                    onclick: move |_| game.write().toggle_canon(canon),
                    "{canon.label()}"
                }
                span { class: "muted", "{summary}" }
            }

            if selected {
                div { class: "segmented scope-mode",
                    button {
                        class: if !choosing_books { "segment active" } else { "segment" },
                        onclick: move |_| game.write().set_canon_book_scope(canon, BookScope::All),
                        "All books"
                    }
                    button {
                        class: if choosing_books { "segment active" } else { "segment" },
                        onclick: move |_| {
                            game.write().set_canon_book_scope(canon, BookScope::Selected(Vec::new()));
                        },
                        "Choose books"
                    }
                }

                if choosing_books {
                    div { class: "scope-book-tools",
                        input {
                            class: "scope-search",
                            placeholder: "Search books",
                            value: "{filter_value}",
                            oninput: move |event| book_filter.set(event.value()),
                        }
                        div { class: "scope-book-actions",
                            button {
                                class: "button secondary",
                                onclick: move |_| {
                                    game.write().set_canon_book_scope(canon, BookScope::Selected(select_all_books.clone()));
                                },
                                "Select all"
                            }
                            button {
                                class: "button secondary",
                                onclick: move |_| {
                                    game.write().set_canon_book_scope(canon, BookScope::Selected(Vec::new()));
                                },
                                "Clear"
                            }
                        }
                    }
                    div { class: "book-select-grid",
                        for book in filtered_books {
                            {
                                let book_name = book.name.clone();
                                let active = selected_books.iter().any(|selected| selected == &book_name);
                                let selected_books = selected_books.clone();
                                rsx! {
                                    button {
                                        class: if active { "choice active" } else { "choice" },
                                        onclick: move |_| {
                                            let mut next_books = selected_books.clone();
                                            if active {
                                                next_books.retain(|selected| selected != &book_name);
                                            } else {
                                                next_books.push(book_name.clone());
                                            }
                                            game.write().set_canon_book_scope(canon, BookScope::Selected(next_books));
                                        },
                                        "{book.name}"
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
