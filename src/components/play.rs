use dioxus::prelude::*;

use super::actions::{request_guess_submission, request_new_game, request_round_advance};
use super::result::ResultPanel;
use super::ui::{HomeButton, PanelHeader, Readout, ToggleButton, use_game};
use crate::api::SnapshotRound;
use crate::game::{GuessPath, GuessStep};

#[component]
pub fn VersePanel() -> Element {
    let game = use_game();
    let game = game.read();
    let Some(session) = game.session() else {
        return rsx! {};
    };

    rsx! {
        section { class: "panel verse-panel",
            div { class: "verse-label",
                span { "Round {session.current_round_index + 1} of {session.rounds.len()}" }
                span { "{game.label()}" }
            }

            if session.finished {
                h2 { "Final score" }
                p { class: "verse", "{session.total_score()} points" }
                RoundSummary { rounds: session.rounds.clone() }
            } else {
                blockquote { class: "verse", "{session.current_round().text}" }
            }
        }
    }
}

#[component]
pub fn GuessPanel() -> Element {
    let finished = use_game()
        .read()
        .session()
        .is_some_and(|session| session.finished);

    rsx! {
        aside { class: "panel picker",
            if finished {
                GameCompletePanel {}
            } else {
                GuessChooser {}
            }
        }
    }
}

#[component]
fn GameCompletePanel() -> Element {
    let game = use_game();
    let navigator = use_navigator();
    let (total, max, loading) = {
        let game = game.read();
        (
            game.total_score(),
            game.max_total_score(),
            game.loading_game,
        )
    };

    rsx! {
        PanelHeader { title: "Game complete", detail: "{total} pts" }
        Readout { label: "Final score", strong { "{total} / {max}" } }
        div { class: "actions",
            button {
                class: "button",
                disabled: loading,
                onclick: move |_| request_new_game(game, navigator),
                "Play again"
            }
            HomeButton {}
        }
    }
}

#[component]
fn GuessChooser() -> Element {
    let mut game = use_game();
    let snapshot = game.read().clone();
    let Some(session) = snapshot.session() else {
        return rsx! {};
    };
    let GuessPath {
        canon,
        book,
        chapter,
        step,
    } = snapshot.guess.clone();
    let round = session.current_round();
    let guessed = round.guess.is_some();
    let scope_canons = snapshot
        .guess_scope()
        .canons
        .iter()
        .map(|scope| scope.canon)
        .collect::<Vec<_>>();
    let book_count = canon.map_or(0, |canon| snapshot.books_for(canon).len());
    let show_canon_crumb = scope_canons.len() > 1;
    let show_book_crumb = book_count > 1;
    let has_visible_crumb =
        show_canon_crumb || (show_book_crumb && book.is_some()) || chapter.is_some();
    let guess = snapshot.guess.chapter_ref();

    rsx! {
        PanelHeader { title: "Guess location", detail: "{step.label()}" }

        if let Some(canon) = canon.filter(|_| has_visible_crumb) {
            Breadcrumbs {
                canon,
                book: book.clone(),
                chapter,
                show_canon: show_canon_crumb,
                show_book: show_book_crumb,
                guessed,
                step,
            }
        }

        match step {
            GuessStep::Canon => rsx! {
                div { class: "grid",
                    for option in scope_canons {
                        ToggleButton {
                            kind: "choice",
                            active: canon == Some(option),
                            disabled: guessed,
                            onclick: move |_| game.write().select_canon(option),
                            "{option.label()}"
                        }
                    }
                }
            },
            GuessStep::Book => rsx! {
                div { class: "grid book-grid",
                    for option in canon.map(|canon| snapshot.books_for(canon)).unwrap_or_default() {
                        {
                            let name = option.name.clone();
                            rsx! {
                                ToggleButton {
                                    kind: "choice",
                                    active: book.as_ref() == Some(&option.name),
                                    disabled: guessed,
                                    onclick: move |_| game.write().select_book(name.clone()),
                                    "{option.name}"
                                }
                            }
                        }
                    }
                }
            },
            GuessStep::Chapter => rsx! {
                div { class: "grid chapter-grid chapter-matrix",
                    for option in canon.zip(book.as_deref()).map(|(canon, book)| snapshot.chapters_for(canon, book)).unwrap_or_default() {
                        ToggleButton {
                            kind: "choice",
                            active: chapter == Some(option),
                            disabled: guessed,
                            onclick: move |_| game.write().guess.select_chapter(option),
                            "{option}"
                        }
                    }
                }
            },
            GuessStep::Ready => rsx! {
                Readout { label: "Ready to submit",
                    if let Some(guess) = guess.as_ref() {
                        strong { "{guess.canon.label()} · {guess}" }
                    }
                }
            },
        }

        if guess.is_some() {
            div { class: "actions",
                button {
                    class: "button",
                    disabled: guessed || snapshot.submitting_guess,
                    onclick: move |_| request_guess_submission(game),
                    if snapshot.submitting_guess { "Submitting" } else { "Submit guess" }
                }
            }
        }

        if let Some(result) = round.guess.clone() {
            ResultPanel { result }
            div { class: "actions",
                button {
                    class: "button",
                    onclick: move |_| request_round_advance(game),
                    if session.is_last_round() { "Finish game" } else { "Next round" }
                }
            }
        }
    }
}

#[component]
fn Breadcrumbs(
    canon: crate::scriptures::Canon,
    book: Option<String>,
    chapter: Option<u16>,
    show_canon: bool,
    show_book: bool,
    guessed: bool,
    step: GuessStep,
) -> Element {
    let mut game = use_game();
    let crumb_class = |current: GuessStep| {
        if step == current {
            "crumb current"
        } else {
            "crumb set"
        }
    };

    rsx! {
        div { class: "breadcrumb-bar",
            nav { class: "breadcrumbs", aria_label: "Guess path",
                if show_canon {
                    button {
                        class: crumb_class(GuessStep::Book),
                        disabled: guessed,
                        onclick: move |_| game.write().guess.open_canon(),
                        "{canon.label()}"
                    }
                }
                if let Some(book) = book.filter(|_| show_book) {
                    if show_canon {
                        span { class: "crumb-separator", "/" }
                    }
                    button {
                        class: crumb_class(GuessStep::Chapter),
                        disabled: guessed,
                        onclick: move |_| game.write().guess.open_book(),
                        "{book}"
                    }
                }
                if let Some(chapter) = chapter {
                    if show_canon || show_book {
                        span { class: "crumb-separator", "/" }
                    }
                    button {
                        class: crumb_class(GuessStep::Ready),
                        disabled: guessed,
                        onclick: move |_| game.write().guess.open_chapter(),
                        "Chapter {chapter}"
                    }
                }
            }
            button {
                class: "clear-guess",
                disabled: guessed,
                aria_label: "Clear guess",
                title: "Clear guess",
                onclick: move |_| game.write().reset_guess(),
                "×"
            }
        }
    }
}

#[component]
fn RoundSummary(rounds: Vec<SnapshotRound>) -> Element {
    rsx! {
        div { class: "round-list",
            for (index, round) in rounds.into_iter().enumerate() {
                div { class: "round-row",
                    strong { "Round {index + 1}" }
                    span {
                        if let Some(guess) = round.guess.as_ref() {
                            "{guess.answer.label()}"
                        } else {
                            "Unanswered"
                        }
                    }
                    span { "{round.guess.as_ref().map_or(0, |guess| guess.score.points)} pts" }
                }
            }
        }
    }
}
