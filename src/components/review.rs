use dioxus::prelude::*;

use super::actions::{request_study_game, unmark_review};
use super::ui::{HomeButton, PanelHeader, Readout, use_game};
use crate::routes::Route;
use crate::stats::ReviewItem;

#[component]
pub fn StatsPanel() -> Element {
    let navigator = use_navigator();
    let (stats, review_count) = {
        let game = use_game();
        let game = game.read();
        (game.account.stats.clone(), game.account.review_items.len())
    };
    let weakest_books = stats.weakest_books(3);

    rsx! {
        div { class: "stats-panel",
            PanelHeader { title: "Stats", detail: "{stats.games_played} games" }

            if stats.games_played == 0 {
                p { class: "muted", "No completed games yet." }
            } else {
                div { class: "stat-grid",
                    div {
                        span { class: "muted", "Best" }
                        if let Some(best) = stats.best_score {
                            strong { "{best.score} / {best.possible_score}" }
                            span { class: "muted", "{best.score_percent()}%" }
                        }
                    }
                    div {
                        span { class: "muted", "Average" }
                        strong { "{stats.average_percent().unwrap_or_default()}%" }
                        span { class: "muted", "{stats.rounds_played} rounds" }
                    }
                    div {
                        span { class: "muted", "Review" }
                        strong { "{review_count}" }
                        span { class: "muted", "marked" }
                    }
                }

                if !weakest_books.is_empty() {
                    div { class: "weak-books",
                        span { class: "setup-label", "Weakest books" }
                        for (book, book_stats) in weakest_books {
                            div { class: "weak-book-row",
                                span { "{book}" }
                                span { class: "muted", "{book_stats.average_percent().unwrap_or_default()}%" }
                            }
                        }
                    }
                }
            }
            if review_count > 0 {
                div { class: "actions stats-actions",
                    button {
                        class: "button secondary",
                        onclick: move |_| {
                            navigator.push(Route::Review {});
                        },
                        "Review marked"
                    }
                }
            }
        }
    }
}

#[component]
pub fn ReviewPanel() -> Element {
    let game = use_game();
    let navigator = use_navigator();
    let mut selected_index = use_signal(|| 0_usize);
    let (items, loading_game) = {
        let game = game.read();
        (game.account.review_items.clone(), game.loading_game)
    };
    let bounded_index = selected_index().min(items.len().saturating_sub(1));
    let selected_item = items.get(bounded_index).cloned();

    rsx! {
        div { class: "review-layout",
            section { class: "panel review-list-panel",
                PanelHeader { title: "Review", detail: "{items.len()} marked" }

                if items.is_empty() {
                    p { class: "muted", "No marked verses yet." }
                } else {
                    div { class: "review-list",
                        for (index, item) in items.iter().enumerate() {
                            button {
                                class: if index == bounded_index { "review-row active" } else { "review-row" },
                                onclick: move |_| selected_index.set(index),
                                strong { "{item.passage.label()}" }
                                span { class: "muted", "{item.score} pts" }
                            }
                        }
                    }
                }

                div { class: "actions",
                    HomeButton {}
                    if !items.is_empty() {
                        button {
                            class: "button",
                            disabled: loading_game,
                            onclick: move |_| {
                                let set = game.read().review_study_set();
                                let round_count = set.passages.len();
                                request_study_game(game, navigator, set, round_count);
                            },
                            if loading_game { "Starting..." } else { "Practice marked" }
                        }
                    }
                }
            }

            aside { class: "panel review-detail-panel",
                if let Some(item) = selected_item {
                    ReviewDetail {
                        item,
                        selected_index: bounded_index,
                        set_selected_index: selected_index,
                    }
                } else {
                    Readout { label: "Review queue", strong { "Nothing marked" } }
                }
            }
        }
    }
}

#[component]
fn ReviewDetail(
    item: ReviewItem,
    selected_index: usize,
    set_selected_index: Signal<usize>,
) -> Element {
    let game = use_game();
    let reference = item.passage.label();

    rsx! {
        div { class: "review-detail",
            PanelHeader { title: "{reference}", detail: "{item.score} pts" }
            blockquote { class: "review-verse", "{item.text}" }
            Readout { label: "Marked answer", strong { "{reference}" } }
            div { class: "actions",
                button {
                    class: "button secondary",
                    onclick: move |_| {
                        unmark_review(game, item.passage.clone());
                        set_selected_index.set(selected_index.saturating_sub(1));
                    },
                    "Unmark"
                }
            }
        }
    }
}
