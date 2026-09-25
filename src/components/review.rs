use dioxus::prelude::*;

use super::request_review_game;
use crate::game::Game;
use crate::routes::Route;
use crate::stats::{ReviewItem, Stats};

#[component]
pub(super) fn StatsPanel(game: Signal<Game>, stats: Stats) -> Element {
    let navigator = use_navigator();
    let review_count = stats.review_items.len();

    rsx! {
        div { class: "stats-panel",
            div { class: "picker-header",
                h2 { "Stats" }
                span { class: "muted", "{stats.games_played} games" }
            }

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
                        strong { "{stats.review_items.len()}" }
                        span { class: "muted", "marked" }
                    }
                }

                if !stats.weakest_books(3).is_empty() {
                    div { class: "weak-books",
                        span { class: "setup-label", "Weakest books" }
                        for (book, book_stats) in stats.weakest_books(3) {
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
pub(super) fn ReviewPanel(game: Signal<Game>) -> Element {
    let navigator = use_navigator();
    let mut selected_index = use_signal(|| 0_usize);
    let snapshot = game.read().clone();
    let items = snapshot.stats.review_items.clone();
    let bounded_index = selected_index().min(items.len().saturating_sub(1));
    let selected_item = items.get(bounded_index).cloned();

    rsx! {
        div { class: "review-layout",
            section { class: "panel review-list-panel",
                div { class: "picker-header",
                    h2 { "Review" }
                    span { class: "muted", "{items.len()} marked" }
                }

                if items.is_empty() {
                    p { class: "muted", "No marked verses yet." }
                } else {
                    div { class: "review-list",
                        for (index, item) in items.iter().cloned().enumerate() {
                            {
                                let reference = item.passage().label();
                                rsx! {
                                    button {
                                        class: if index == bounded_index { "review-row active" } else { "review-row" },
                                        onclick: move |_| selected_index.set(index),
                                        strong { "{reference}" }
                                        span { class: "muted", "{item.score} pts" }
                                    }
                                }
                            }
                        }
                    }
                }

                div { class: "actions",
                    button {
                        class: "button secondary",
                        onclick: move |_| {
                            game.write().change_settings();
                            navigator.push(Route::Setup {});
                        },
                        "Home"
                    }
                    if !items.is_empty() {
                        button {
                            class: "button",
                            disabled: snapshot.loading_game,
                            onclick: move |_| request_review_game(game, navigator),
                            if snapshot.loading_game {
                                "Starting..."
                            } else {
                                "Practice marked"
                            }
                        }
                    }
                }
            }

            aside { class: "panel review-detail-panel",
                if let Some(item) = selected_item {
                    ReviewDetail {
                        game,
                        item,
                        selected_index: bounded_index,
                        set_selected_index: selected_index,
                    }
                } else {
                    div { class: "ready",
                        span { class: "muted", "Review queue" }
                        strong { "Nothing marked" }
                    }
                }
            }
        }
    }
}

#[component]
fn ReviewDetail(
    game: Signal<Game>,
    item: ReviewItem,
    selected_index: usize,
    set_selected_index: Signal<usize>,
) -> Element {
    let remove_passage = item.passage();
    let reference = remove_passage.label();

    rsx! {
        div { class: "review-detail",
            div { class: "picker-header",
                h2 { "{reference}" }
                span { class: "muted", "{item.score} pts" }
            }
            blockquote { class: "review-verse", "{item.text}" }
            div { class: "ready",
                span { class: "muted", "Marked answer" }
                strong { "{reference}" }
            }
            div { class: "actions",
                button {
                    class: "button secondary",
                    onclick: move |_| {
                        game.write().remove_review_item(&remove_passage);
                        set_selected_index.set(selected_index.saturating_sub(1));
                    },
                    "Unmark"
                }
            }
        }
    }
}
