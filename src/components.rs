use dioxus::prelude::*;

use crate::game::{Game, GuessResult, GuessStep, Round, Screen};
use crate::loader::{create_game, load_metadata, submit_guess};
use crate::scriptures::{Canon, Difficulty, GameMode};
use crate::stats::Stats;

#[component]
pub fn App() -> Element {
    let mut game = use_signal(Game::new);
    let mut metadata_resource = use_resource(move || async move {
        let snapshot = game.read().clone();
        if snapshot.screen == Screen::Setup && !snapshot.metadata_ready() {
            load_metadata(snapshot.metadata_request()).await.map(Some)
        } else {
            Ok(None)
        }
    });

    use_effect(move || {
        let Some(Ok(Some(metadata))) = metadata_resource.value().read().as_ref().cloned() else {
            return;
        };

        game.write().apply_metadata(metadata);
        metadata_resource.clear();
    });

    let snapshot = game.read().clone();
    let load_error = metadata_resource
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
                        if snapshot.metadata_ready() {
                            span { "{snapshot.settings.selection_label()} · {snapshot.settings.round_count} rounds · {snapshot.settings.difficulty.label()} · {snapshot.verse_count_for_difficulty()} of {snapshot.total_verse_count()} verses in play" }
                        } else {
                            span { "{snapshot.settings.selection_label()} · loading game data" }
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

fn request_new_game(mut game: Signal<Game>) {
    let request = game.read().new_game_request();
    game.write().begin_starting_game();

    spawn(async move {
        match create_game(request).await {
            Ok(response) => game.write().start_game(response),
            Err(error) => game.write().fail_request(error),
        }
    });
}

fn request_guess_submission(mut game: Signal<Game>) {
    let game_id = game.read().game_id.clone();
    let request = game.read().guess_request();
    let (Some(game_id), Some(request)) = (game_id, request) else {
        return;
    };

    game.write().begin_submitting_guess();
    spawn(async move {
        match submit_guess(&game_id, request).await {
            Ok(response) => game.write().apply_guess(response),
            Err(error) => game.write().fail_request(error),
        }
    });
}

#[component]
fn VersePanel(game: Signal<Game>) -> Element {
    let snapshot = game.read().clone();
    let round_number = snapshot.current_round_index + 1;
    let mode_label = snapshot.settings.selection_label();

    rsx! {
        section { class: "panel verse-panel",
            div { class: "verse-label",
                span { "Round {round_number} of {snapshot.settings.round_count}" }
                span { "{mode_label}" }
            }

            if snapshot.finished {
                h2 { "Final score" }
                p { class: "verse", "{snapshot.total_score()} points" }
                RoundSummary { rounds: snapshot.rounds.clone() }
            } else {
                blockquote { class: "verse", "{snapshot.current_round().text}" }
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
                disabled: snapshot.loading_game,
                onclick: move |_| request_new_game(game),
                "Play again"
            }
            button {
                class: "button secondary",
                onclick: move |_| game.write().change_settings(),
                "Home"
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
    let canon_count = snapshot.settings.canons.len();
    let book_count = selected_canon
        .map(|canon| snapshot.books_for(canon).len())
        .unwrap_or_default();
    let show_canon_crumb = canon_count > 1;
    let show_book_crumb = book_count > 1;
    let has_visible_crumb = show_canon_crumb
        || (show_book_crumb && selected_book.is_some())
        || selected_chapter.is_some();
    let can_submit =
        selected_canon.is_some() && selected_book.is_some() && selected_chapter.is_some();

    rsx! {
        div { class: "picker-header",
            h2 { "Guess location" }
            span { class: "muted", "{step.label()}" }
        }

        if selected_canon.is_some() && has_visible_crumb {
            Breadcrumbs {
                game,
                canon: selected_canon.unwrap(),
                selected_book: selected_book.clone(),
                selected_chapter,
                show_canon: show_canon_crumb,
                show_book: show_book_crumb,
                guessed,
                step,
            }
        }

        match step {
            GuessStep::Canon => rsx! {
                div { class: "grid",
                    for canon in snapshot.settings.canons.iter().copied() {
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
                    for book in snapshot.books_for(selected_canon.expect("canon selected before book step")) {
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
                let canon = selected_canon.expect("canon selected before chapter step");
                rsx! {
                    div { class: "grid chapter-grid",
                        for chapter in snapshot.chapters_for(canon, &book_name) {
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
                        if let Some(canon) = selected_canon {
                            strong { "{canon.label()} · {book} {chapter}" }
                        }
                    }
                }
            }
        }

        if can_submit {
            div { class: "actions",
                button {
                    class: "button",
                    disabled: guessed || snapshot.submitting_guess,
                    onclick: move |_| request_guess_submission(game),
                    if snapshot.submitting_guess { "Submitting" } else { "Submit guess" }
                }
            }
        }

        if let Some(result) = snapshot.current_round().guess.clone() {
            ResultPanel {
                result: result,
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
    show_canon: bool,
    show_book: bool,
    guessed: bool,
    step: GuessStep,
) -> Element {
    rsx! {
        div { class: "breadcrumb-bar",
            nav { class: "breadcrumbs", aria_label: "Guess path",
                if show_canon {
                    button {
                        class: if step == GuessStep::Book { "crumb current" } else { "crumb set" },
                        disabled: guessed,
                        onclick: move |_| game.write().open_canon(),
                        "{canon.label()}"
                    }
                }
                if let Some(book) = selected_book {
                    if show_book {
                        if show_canon {
                            span { class: "crumb-separator", "/" }
                        }
                        button {
                            class: if step == GuessStep::Chapter { "crumb current" } else { "crumb set" },
                            disabled: guessed,
                            onclick: move |_| game.write().open_book(),
                            "{book}"
                        }
                    }
                }
                if let Some(chapter) = selected_chapter {
                    if show_canon || show_book {
                        span { class: "crumb-separator", "/" }
                    }
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
    let metadata_ready = snapshot.metadata_ready();

    rsx! {
        div { class: "setup-layout",
            section { class: "panel setup-panel",
                div { class: "picker-header",
                    h2 { "New game" }
                    span { class: "muted", "{snapshot.settings.selection_label()}" }
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
                    span { class: "setup-label", "Presets" }
                    div { class: "grid",
                        for mode in GameMode::ALL {
                            button {
                                class: if mode.canons() == snapshot.settings.canons.as_slice() { "choice active" } else { "choice" },
                                onclick: move |_| game.write().set_preset(mode),
                                "{mode.label()}"
                            }
                        }
                    }
                }

                div { class: "setup-group",
                    span { class: "setup-label", "Canons" }
                    div { class: "grid",
                        for canon in Canon::ALL {
                            {
                                let selected = snapshot.settings.canons.contains(&canon);
                                rsx! {
                                    button {
                                        class: if selected { "choice active" } else { "choice" },
                                        onclick: move |_| game.write().toggle_canon(canon),
                                        "{canon.label()}"
                                    }
                                }
                            }
                        }
                    }
                }

                div { class: "actions",
                    button {
                        class: "button",
                        disabled: !metadata_ready || snapshot.loading_game,
                        onclick: move |_| request_new_game(game),
                        if snapshot.loading_game { "Starting" } else { "Start" }
                    }
                }
                if let Some(error) = snapshot.error.as_ref() {
                    div { class: "callout warning",
                        strong { "Game unavailable" }
                        span { "{error}" }
                    }
                }
            }

            aside { class: "panel setup-side",
                div { class: "ready",
                    span { class: "muted", "Verse pool" }
                    if metadata_ready {
                        strong { "{snapshot.verse_count_for_difficulty()} of {snapshot.total_verse_count()} verses" }
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

                StatsPanel { stats: snapshot.stats.clone() }
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
fn ResultPanel(result: GuessResult) -> Element {
    let mut reader_open = use_signal(|| false);
    let label = result.score.distance_label();
    let chapter_title = format!("{} {}", result.answer.book, result.answer.chapter);

    rsx! {
        div { class: "result",
            h2 { "Result" }
            p { class: "score", "{result.score.points}" }
            p { class: "distance", "{label}" }
            div { class: "result-grid",
                div {
                    span { class: "muted", "Actual" }
                    strong { "{result.answer.book} {result.answer.chapter}:{result.answer.verse}" }
                }
                div {
                    span { class: "muted", "Guess" }
                    strong { "{result.guess.book} {result.guess.chapter}" }
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
                    answer_verse: result.answer.verse,
                    verses: result.chapter_verses.clone(),
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
    verses: Vec<crate::api::ChapterVerse>,
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
                            class: if verse.verse == answer_verse { "chapter-verse answer-verse" } else { "chapter-verse" },
                            sup { "{verse.verse}" }
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
                    span {
                        if let Some(guess) = round.guess.as_ref() {
                            "{guess.answer.book} {guess.answer.chapter}:{guess.answer.verse}"
                        } else {
                            "Unanswered"
                        }
                    }
                    span { "{round.guess.as_ref().map(|guess| guess.score.points).unwrap_or_default()} pts" }
                }
            }
        }
    }
}
