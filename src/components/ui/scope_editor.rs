use dioxus::prelude::*;

use super::ToggleButton;
use crate::api::CanonMetadata;
use crate::game::catalog;
use crate::scriptures::{BookInfo, BookScope, Canon, GameScope};

#[component]
pub fn ScopeEditor(
    class: &'static str,
    scope: GameScope,
    catalog: Vec<CanonMetadata>,
    on_change: EventHandler<GameScope>,
) -> Element {
    rsx! {
        div { class,
            for canon in Canon::ALL {
                ScopeCanonRow {
                    key: "{canon:?}",
                    scope: scope.clone(),
                    canon,
                    books: catalog::books(&catalog, canon).to_vec(),
                    on_change,
                }
            }
        }
    }
}

#[component]
fn ScopeCanonRow(
    scope: ReadSignal<GameScope>,
    canon: Canon,
    books: Vec<BookInfo>,
    on_change: EventHandler<GameScope>,
) -> Element {
    let mut book_filter = use_signal(String::new);
    let book_scope = scope
        .read()
        .canon_scope(canon)
        .map(|canon_scope| canon_scope.books.clone());
    let selected = book_scope.is_some();
    let summary = book_scope
        .as_ref()
        .map_or_else(|| "Off".to_string(), BookScope::summary);
    let filter = book_filter().trim().to_lowercase();
    let visible_books = books
        .iter()
        .filter(|book| filter.is_empty() || book.name.to_lowercase().contains(&filter))
        .collect::<Vec<_>>();
    let all_book_names = books
        .iter()
        .map(|book| book.name.clone())
        .collect::<Vec<_>>();
    let edit = move |change: &dyn Fn(&mut GameScope)| {
        let mut next = scope();
        change(&mut next);
        on_change.call(next);
    };

    rsx! {
        div { class: if selected { "scope-canon selected" } else { "scope-canon" },
            div { class: "scope-canon-header",
                ToggleButton {
                    kind: "choice",
                    active: selected,
                    onclick: move |_| edit(&|scope| scope.toggle_canon(canon)),
                    "{canon.label()}"
                }
                span { class: "muted", "{summary}" }
            }

            if let Some(book_scope) = book_scope {
                div { class: "segmented scope-mode",
                    ToggleButton {
                        active: book_scope == BookScope::All,
                        onclick: move |_| edit(&|scope| scope.set_books(canon, BookScope::All)),
                        "All books"
                    }
                    ToggleButton {
                        active: book_scope != BookScope::All,
                        onclick: move |_| edit(&|scope| scope.set_books(canon, BookScope::Selected(Vec::new()))),
                        "Choose books"
                    }
                }

                if book_scope != BookScope::All {
                    div { class: "scope-book-tools",
                        input {
                            class: "scope-search",
                            placeholder: "Search books",
                            value: "{book_filter}",
                            oninput: move |event| book_filter.set(event.value()),
                        }
                        div { class: "scope-book-actions",
                            button {
                                class: "button secondary",
                                onclick: move |_| edit(&|scope| scope.set_books(canon, BookScope::Selected(all_book_names.clone()))),
                                "Select all"
                            }
                            button {
                                class: "button secondary",
                                onclick: move |_| edit(&|scope| scope.set_books(canon, BookScope::Selected(Vec::new()))),
                                "Clear"
                            }
                        }
                    }
                    div { class: "book-select-grid",
                        for book in visible_books {
                            {
                                let name = book.name.clone();
                                rsx! {
                                    ToggleButton {
                                        kind: "choice",
                                        active: book_scope.includes(&book.name),
                                        onclick: move |_| edit(&|scope| scope.toggle_book(canon, &name)),
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
