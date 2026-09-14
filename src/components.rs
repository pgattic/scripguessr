use dioxus::prelude::*;

use crate::game::{Game, GuessResult, GuessStep, Round, Screen};
use crate::loader::load_scriptures;
use crate::scriptures::{Canon, Difficulty, Reference, Verse};
use crate::stats::Stats;

#[component]
pub fn App() -> Element {
    let mut game = use_signal(Game::new);
    let selected_canon = game.read().settings.canon;
    let canon_loaded = game.read().selected_canon_loaded();
    let mut scripture_resource = use_resource(move || async move {
        if canon_loaded {
            Ok(None)
        } else {
            load_scriptures(selected_canon).await.map(Some)
        }
    });

    use_effect(move || {
        let Some(Ok(Some((canon, scriptures)))) =
            scripture_resource.value().read().as_ref().cloned()
        else {
            return;
        };

        game.write().set_scriptures(canon, scriptures);
        scripture_resource.clear();
    });

    let snapshot = game.read().clone();
    let load_error = scripture_resource
        .value()
        .read()
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());

    rsx! {
        document::Stylesheet {
            href: asset!("/assets/main.css")
        }
        main { class: "app",
            div { class: "shell",
                header { class: "topbar",
                    div { class: "brand",
                        h1 { "ScripGuessr" }
                        if let Some(scriptures) = snapshot.selected_scriptures() {
                            span { "{snapshot.settings.canon.label()} · {snapshot.settings.round_count} rounds · {snapshot.settings.difficulty.label()} · {scriptures.verse_count_for_difficulty(snapshot.settings.difficulty)} of {scriptures.total_verse_count()} verses in play" }
                        } else {
                            span { "{snapshot.settings.canon.label()} · loading scripture data" }
                        }
                    }
                    if snapshot.screen == Screen::Playing {
                        div { class: "pill", "Total {snapshot.total_score()} / {snapshot.max_total_score()}" }
                    }
                }

                if snapshot.screen == Screen::Setup {
                    SetupPanel { game: game, load_error }
                } else {
                    div { class: "layout",
                        VersePanel { game: game }
                        GuessPanel { game: game }
                    }
                }
            }
        }
    }
}

#[component]
fn VersePanel(game: Signal<Game>) -> Element {
    let snapshot = game.read().clone();
    let round_number = snapshot.current_round_index + 1;
    let canon_label = snapshot.settings.canon.label();

    rsx! {
        section { class: "panel verse-panel",
            div { class: "verse-label",
                span { "Round {round_number} of {snapshot.settings.round_count}" }
                span { "{canon_label}" }
            }

            if snapshot.finished {
                h2 { "Final score" }
                p { class: "verse", "{snapshot.total_score()} points" }
                RoundSummary { rounds: snapshot.rounds.clone() }
            } else {
                blockquote { class: "verse", "{snapshot.current_round().verse.text}" }
            }
        }
    }
}

#[component]
fn GuessPanel(game: Signal<Game>) -> Element {
    let snapshot = game.read().clone();

    rsx! {
        aside { class: "panel picker",
            if snapshot.finished {
                GameCompletePanel { game, snapshot }
            } else {
                GuessChooser { game, snapshot }
            }
        }
    }
}

#[component]
fn GameCompletePanel(game: Signal<Game>, snapshot: Game) -> Element {
    rsx! {
        div { class: "picker-header",
            h2 { "Game complete" }
            span { class: "muted", "{snapshot.total_score()} pts" }
        }
        div { class: "ready",
            span { class: "muted", "Final score" }
            strong { "{snapshot.total_score()} / {snapshot.max_total_score()}" }
        }
        if snapshot.last_game_new_best {
            div { class: "callout",
                strong { "New best" }
                span { "That score tops your saved history." }
            }
        }
        div { class: "actions",
            button {
                class: "button",
                onclick: move |_| game.write().restart(),
                "Play again"
            }
            button {
                class: "button secondary",
                onclick: move |_| game.write().change_settings(),
                "Change settings"
            }
        }
    }
}

#[component]
fn GuessChooser(game: Signal<Game>, snapshot: Game) -> Element {
    let selected_canon = snapshot.selected_canon;
    let selected_book = snapshot.selected_book.clone();
    let selected_chapter = snapshot.selected_chapter;
    let guessed = snapshot.current_round().guess.is_some();
    let step = snapshot.active_step;
    let can_submit =
        selected_canon.is_some() && selected_book.is_some() && selected_chapter.is_some();

    rsx! {
        div { class: "picker-header",
            h2 { "Guess location" }
            span { class: "muted", "{step.label()}" }
        }

        if selected_canon.is_some() {
            Breadcrumbs {
                game,
                canon: snapshot.settings.canon,
                selected_book: selected_book.clone(),
                selected_chapter,
                guessed,
                step,
            }
        }

        match step {
            GuessStep::Canon => rsx! {
                div { class: "grid",
                    for canon in [snapshot.settings.canon] {
                        button {
                            class: if selected_canon == Some(canon) { "choice active" } else { "choice" },
                            disabled: guessed,
                            onclick: move |_| game.write().select_canon(canon),
                            "{canon.label()}"
                        }
                    }
                }
            },
            GuessStep::Book => rsx! {
                div { class: "grid book-grid",
                    for book in snapshot.scriptures().books.clone() {
                        {
                            let book_name = book.name.clone();
                            let active = selected_book.as_ref() == Some(&book_name);
                            rsx! {
                                button {
                                    class: if active { "choice active" } else { "choice" },
                                    disabled: guessed,
                                    onclick: move |_| game.write().select_book(book_name.clone()),
                                    "{book.name}"
                                }
                            }
                        }
                    }
                }
            },
            GuessStep::Chapter => {
                let book_name = selected_book.clone().unwrap_or_default();
                rsx! {
                    div { class: "grid chapter-grid",
                        for chapter in snapshot.chapters_for(&book_name) {
                            button {
                                class: if selected_chapter == Some(chapter) { "choice active" } else { "choice" },
                                disabled: guessed,
                                onclick: move |_| game.write().select_chapter(chapter),
                                "{chapter}"
                            }
                        }
                    }
                }
            },
            GuessStep::Ready => rsx! {
                div { class: "ready",
                    span { class: "muted", "Ready to submit" }
                    if let (Some(book), Some(chapter)) = (selected_book.clone(), selected_chapter) {
                        strong { "{book} {chapter}" }
                    }
                }
            }
        }

        if can_submit {
            div { class: "actions",
                button {
                    class: "button",
                    disabled: guessed,
                    onclick: move |_| game.write().submit_guess(),
                    "Submit guess"
                }
            }
        }

        if let Some(result) = snapshot.current_round().guess.clone() {
            ResultPanel {
                result: result,
                answer: snapshot.current_round().verse.reference.clone(),
                chapter_verses: snapshot.scriptures().verses_for_chapter(&snapshot.current_round().verse.reference),
            }
            div { class: "actions",
                button {
                    class: "button",
                    onclick: move |_| game.write().next_round(),
                    if snapshot.is_last_round() { "Finish game" } else { "Next round" }
                }
            }
        }
    }
}

#[component]
fn Breadcrumbs(
    game: Signal<Game>,
    canon: Canon,
    selected_book: Option<String>,
    selected_chapter: Option<u16>,
    guessed: bool,
    step: GuessStep,
) -> Element {
    rsx! {
        div { class: "breadcrumb-bar",
            nav { class: "breadcrumbs", aria_label: "Guess path",
                button {
                    class: if step == GuessStep::Book { "crumb current" } else { "crumb set" },
                    disabled: guessed,
                    onclick: move |_| game.write().open_canon(),
                    "{canon.label()}"
                }
                if let Some(book) = selected_book {
                    span { class: "crumb-separator", "/" }
                    button {
                        class: if step == GuessStep::Chapter { "crumb current" } else { "crumb set" },
                        disabled: guessed,
                        onclick: move |_| game.write().open_book(),
                        "{book}"
                    }
                }
                if let Some(chapter) = selected_chapter {
                    span { class: "crumb-separator", "/" }
                    button {
                        class: if step == GuessStep::Ready { "crumb current" } else { "crumb set" },
                        disabled: guessed,
                        onclick: move |_| game.write().open_chapter(),
                        "Chapter {chapter}"
                    }
                }
            }
            button {
                class: "clear-guess",
                disabled: guessed,
                aria_label: "Clear guess",
                title: "Clear guess",
                onclick: move |_| game.write().clear_guess(),
                "×"
            }
        }
    }
}

#[component]
fn SetupPanel(game: Signal<Game>, load_error: Option<String>) -> Element {
    let snapshot = game.read().clone();
    let scriptures = snapshot.selected_scriptures();
    let canon_loaded = scriptures.is_some();

    rsx! {
        section { class: "panel setup-panel",
            div { class: "picker-header",
                h2 { "New game" }
                span { class: "muted", "{snapshot.settings.canon.label()}" }
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

            div { class: "setup-group",
                span { class: "setup-label", "Canon" }
                div { class: "grid",
                    for canon in Canon::ALL {
                        button {
                            class: if snapshot.settings.canon == canon { "choice active" } else { "choice" },
                            onclick: move |_| game.write().set_canon(canon),
                            "{canon.label()}"
                        }
                    }
                }
            }

            div { class: "ready",
                span { class: "muted", "Verse pool" }
                if let Some(scriptures) = scriptures {
                    strong { "{scriptures.verse_count_for_difficulty(snapshot.settings.difficulty)} of {scriptures.total_verse_count()} verses" }
                } else if load_error.is_some() {
                    strong { "Could not load {snapshot.settings.canon.label()}" }
                } else {
                    strong { "Loading {snapshot.settings.canon.label()}" }
                }
            }

            if let Some(error) = load_error {
                div { class: "callout warning",
                    strong { "Scripture data unavailable" }
                    span { "{error}" }
                }
            }

            StatsPanel { stats: snapshot.stats.clone() }

            div { class: "actions",
                button {
                    class: "button",
                    disabled: !canon_loaded,
                    onclick: move |_| game.write().start_game(),
                    "Start"
                }
            }
        }
    }
}

#[component]
fn StatsPanel(stats: Stats) -> Element {
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
        }
    }
}

#[component]
fn ResultPanel(result: GuessResult, answer: Reference, chapter_verses: Vec<Verse>) -> Element {
    let mut reader_open = use_signal(|| false);
    let label = result.score.distance_label();
    let chapter_title = format!("{} {}", answer.book, answer.chapter);

    rsx! {
        div { class: "result",
            h2 { "Result" }
            p { class: "score", "{result.score.points}" }
            p { class: "distance", "{label}" }
            div { class: "result-grid",
                div {
                    span { class: "muted", "Actual" }
                    strong { "{answer.book} {answer.chapter}:{answer.verse}" }
                }
                div {
                    span { class: "muted", "Guess" }
                    strong { "{result.book} {result.chapter}" }
                }
            }
            div { class: "actions compact-actions",
                button {
                    class: "button secondary",
                    onclick: move |_| reader_open.set(true),
                    "Read chapter"
                }
            }
            if reader_open() {
                ChapterReader {
                    title: chapter_title.clone(),
                    answer_verse: answer.verse,
                    verses: chapter_verses.clone(),
                    on_close: move |_| reader_open.set(false),
                }
            }
        }
    }
}

#[component]
fn ChapterReader(
    title: String,
    answer_verse: u16,
    verses: Vec<Verse>,
    on_close: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "dialog-backdrop",
            div { class: "chapter-dialog", role: "dialog", aria_modal: "true",
                div { class: "dialog-header",
                    h2 { "{title}" }
                    button {
                        class: "clear-guess",
                        aria_label: "Close chapter reader",
                        title: "Close",
                        onclick: move |event| on_close.call(event),
                        "×"
                    }
                }
                div { class: "chapter-reader",
                    for verse in verses {
                        p {
                            class: if verse.reference.verse == answer_verse { "chapter-verse answer-verse" } else { "chapter-verse" },
                            sup { "{verse.reference.verse}" }
                            "{verse.text}"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn RoundSummary(rounds: Vec<Round>) -> Element {
    rsx! {
        div { class: "round-list",
            for (index, round) in rounds.into_iter().enumerate() {
                div { class: "round-row",
                    strong { "Round {index + 1}" }
                    span { "{round.verse.reference.book} {round.verse.reference.chapter}:{round.verse.reference.verse}" }
                    span { "{round.guess.as_ref().map(|guess| guess.score.points).unwrap_or_default()} pts" }
                }
            }
        }
    }
}
