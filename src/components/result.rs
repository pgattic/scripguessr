use dioxus::prelude::*;

use super::actions::toggle_review;
use super::ui::{ChapterReader, Readout, use_game};
use crate::api::GuessResponse;
use crate::scoring::{MAX_SCORE, percent};

#[component]
pub fn ResultPanel(result: GuessResponse) -> Element {
    let game = use_game();
    let mut reader_open = use_signal(|| false);
    let distance = if result.answer.canon == result.guess.canon {
        result.score.distance_label()
    } else {
        "Wrong canon".to_string()
    };
    let score_percent = percent(result.score.points, MAX_SCORE).unwrap_or(0);
    let marked_for_review = game.read().current_result_marked_for_review();
    let signed_in = game.read().account.user.is_some();
    let (feedback_title, feedback_detail) = result_feedback(&result);

    rsx! {
        div { class: "result",
            h2 { "Result" }
            p { class: "score", "{result.score.points}" }
            p { class: "distance", "{distance}" }
            div { class: "score-meter", aria_label: "Round score percent",
                div {
                    class: "score-meter-fill",
                    style: "--score-width: {score_percent}%;",
                }
            }
            div { class: "result-feedback",
                strong { "{feedback_title}" }
                span { "{feedback_detail}" }
            }
            div { class: "result-grid",
                div {
                    span { class: "muted", "Actual" }
                    strong { "{result.answer.label()}" }
                }
                div {
                    span { class: "muted", "Guess" }
                    strong { "{result.guess}" }
                }
            }
            if result.source_passage != result.answer {
                Readout { label: "Study passage", class: "result-source",
                    strong { "{result.source_passage.label()}" }
                }
            }
            div { class: "actions compact-actions",
                button {
                    class: "button secondary",
                    onclick: move |_| reader_open.set(true),
                    "Read chapter"
                }
                button {
                    class: if marked_for_review { "button secondary review-active" } else { "button secondary" },
                    disabled: !signed_in,
                    onclick: move |_| toggle_review(game),
                    if !signed_in { "Sign in to mark" } else if marked_for_review { "Marked" } else { "Mark for review" }
                }
            }
            if reader_open() {
                ChapterReader {
                    title: result.answer.chapter_ref().to_string(),
                    highlight: result.answer.verses.clone(),
                    verses: result.chapter_verses.clone(),
                    on_close: move |_| reader_open.set(false),
                }
            }
        }
    }
}

fn result_feedback(result: &GuessResponse) -> (&'static str, String) {
    if result.answer.canon != result.guess.canon {
        return (
            "Different canon",
            "The answer was in a different canon.".to_string(),
        );
    }

    if result.score.chapter_distance == 0 {
        return (
            "Exact match",
            "You placed the verse in the right book and chapter.".to_string(),
        );
    }

    let distance = result.score.distance_label().to_lowercase();
    if result.answer.book == result.guess.book {
        (
            "Same book",
            format!("Your guess was {distance} in the same book."),
        )
    } else {
        ("Same canon", format!("Your guess was {distance}."))
    }
}
