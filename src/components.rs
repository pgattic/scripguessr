use dioxus::prelude::*;

use crate::game::{Game, GuessResult, GuessStep, Round, Screen};
use crate::loader::{create_game, load_metadata, submit_guess};
use crate::scoring::MAX_SCORE;
use crate::scriptures::{BookScope, Canon, Difficulty, GameMode, Reference};
use crate::stats::{ReviewItem, Stats};

#[component]
pub fn App() -> Element {
    let mut game = use_signal(Game::new);
    let mut metadata_resource = use_resource(move || async move {
        let snapshot = game.read().clone();
        if snapshot.screen == Screen::Setup && !snapshot.metadata_current() {
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
                        if snapshot.settings.scope.canons.is_empty() {
                            span { "{snapshot.settings.selection_label()} · choose a scope to start" }
                        } else if snapshot.metadata_ready() {
                            span { "{snapshot.settings.selection_label()} · {snapshot.settings.round_count} rounds · {snapshot.settings.difficulty.label()} · {snapshot.verse_count_for_difficulty()} of {snapshot.total_verse_count()} verses in play" }
                        } else {
                            span { "{snapshot.settings.selection_label()} · loading game data" }
                        }
                    }
                    match snapshot.screen {
                        Screen::Playing => rsx! {
                            div { class: "pill", "Total {snapshot.total_score()} / {snapshot.max_total_score()}" }
                        },
                        Screen::Review => rsx! {
                            div { class: "pill", "{snapshot.stats.review_items.len()} marked" }
                        },
                        Screen::Setup => rsx! {},
                    }
                }

                match snapshot.screen {
                    Screen::Setup => rsx! {
                        SetupPanel { game: game, load_error }
                    },
                    Screen::Playing => rsx! {
                        div { class: "layout",
                            VersePanel { game: game }
                            GuessPanel { game: game }
                        }
                    },
                    Screen::Review => rsx! {
                        ReviewPanel { game: game }
                    },
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
    let canon_count = snapshot.settings.scope.canons.len();
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
                    for canon in snapshot.settings.scope.canons.iter().map(|scope| scope.canon) {
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
                game,
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
                        onclick: move |_| request_new_game(game),
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

#[component]
fn StatsPanel(game: Signal<Game>, stats: Stats) -> Element {
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
                        onclick: move |_| game.write().open_review(),
                        "Review marked"
                    }
                }
            }
        }
    }
}

#[component]
fn ReviewPanel(game: Signal<Game>) -> Element {
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
                                let reference = reference_label(&item.reference);
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
                        onclick: move |_| game.write().change_settings(),
                        "Home"
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
    let reference = reference_label(&item.reference);
    let remove_reference = item.reference.clone();

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
                        game.write().remove_review_item(&remove_reference);
                        set_selected_index.set(selected_index.saturating_sub(1));
                    },
                    "Unmark"
                }
            }
        }
    }
}

fn reference_label(reference: &Reference) -> String {
    format!(
        "{} {}:{}",
        reference.book, reference.chapter, reference.verse
    )
}

#[component]
fn ResultPanel(game: Signal<Game>, result: GuessResult) -> Element {
    let mut reader_open = use_signal(|| false);
    let label = result.score.distance_label();
    let chapter_title = format!("{} {}", result.answer.book, result.answer.chapter);
    let score_percent = result.score.points.saturating_mul(100) / MAX_SCORE;
    let marked_for_review = game.read().current_result_marked_for_review();
    let (feedback_title, feedback_detail) = result_feedback(&result);

    rsx! {
        div { class: "result",
            h2 { "Result" }
            p { class: "score", "{result.score.points}" }
            p { class: "distance", "{label}" }
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
                button {
                    class: if marked_for_review { "button secondary review-active" } else { "button secondary" },
                    onclick: move |_| {
                        game.write().toggle_current_result_review();
                    },
                    if marked_for_review { "Marked" } else { "Mark for review" }
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

fn result_feedback(result: &GuessResult) -> (&'static str, String) {
    if result.score.chapter_distance == 0 {
        return (
            "Exact match",
            "You placed the verse in the right book and chapter.".to_string(),
        );
    }

    let distance = result.score.distance_label().to_lowercase();
    if result.answer.canon == result.guess.canon && result.answer.book == result.guess.book {
        (
            "Same book",
            format!("Your guess was {distance} in the same book."),
        )
    } else if result.answer.canon == result.guess.canon {
        ("Same canon", format!("Your guess was {distance}."))
    } else {
        ("Different canon", format!("Your guess was {distance}."))
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
