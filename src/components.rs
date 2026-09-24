use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::closure::Closure;

use crate::game::{Game, GuessResult, GuessStep, Round, Screen};
use crate::loader::{create_game, load_metadata, submit_guess};
use crate::routes::Route;
use crate::scoring::MAX_SCORE;
use crate::scriptures::Canon;
use crate::study_sets::{StudyPassage, StudySet};

mod atlas;
mod review;
mod setup;
mod study_sets;

use atlas::AtlasPanel;
use review::{ReviewPanel, StatsPanel};
use setup::SetupPanel;
use study_sets::StudySetsPanel;

#[component]
pub fn App() -> Element {
    let game = use_signal(Game::new);

    use_context_provider(|| game);

    rsx! {
        document::Stylesheet {
            href: asset!("/assets/main.css")
        }
        document::Meta {
            name: "viewport",
            content: "width=device-width, initial-scale=1"
        }
        Router::<Route> {}
    }
}

#[component]
pub fn AppShell() -> Element {
    let game = use_context::<Signal<Game>>();
    let snapshot = game.read().clone();
    let route = use_route::<Route>();
    let playing = snapshot.screen == Screen::Playing;
    let subtitle = if playing {
        snapshot
            .active_game
            .as_ref()
            .and_then(|active| active.label.clone())
            .unwrap_or_else(|| snapshot.settings.selection_label())
    } else {
        match route {
            Route::Atlas {} => "Book of Mormon Atlas · people, narratives, and events".to_string(),
            Route::Review {} => "Marked passages".to_string(),
            Route::StudyIndex {} | Route::StudySet { .. } => {
                "Curated and custom study sets".to_string()
            }
            Route::Setup {} | Route::NotFound { .. } => setup_subtitle(&snapshot),
        }
    };

    rsx! {
        main { class: "app",
            div { class: "shell",
                header { class: "topbar",
                    div { class: "brand",
                        h1 { "ScripGuessr" }
                        span { "{subtitle}" }
                    }
                    if playing {
                        div { class: "pill", "Total {snapshot.total_score()} / {snapshot.max_total_score()}" }
                    } else {
                        match route {
                            Route::Review {} => rsx! {
                                div { class: "pill", "{snapshot.stats.review_items.len()} marked" }
                            },
                            Route::StudyIndex {} | Route::StudySet { .. } => rsx! {
                                div { class: "pill", "{snapshot.custom_study_sets.sets.len()} custom" }
                            },
                            Route::Atlas {} => rsx! {
                                div { class: "pill", "239 chapters" }
                            },
                            Route::Setup {} | Route::NotFound { .. } => rsx! {},
                        }
                    }
                }

                if playing {
                    div { class: "layout",
                        VersePanel { game }
                        GuessPanel { game }
                    }
                } else {
                    Outlet::<Route> {}
                }
            }
        }
    }
}

fn setup_subtitle(game: &Game) -> String {
    let label = game.settings.selection_label();
    if game.settings.scope.canons.is_empty() {
        format!("{label} · choose a scope to start")
    } else if game.metadata_ready() {
        format!(
            "{label} · {} rounds · {} · {} of {} verses in play",
            game.settings.round_count,
            game.settings.difficulty.label(),
            game.verse_count_for_difficulty(),
            game.total_verse_count(),
        )
    } else {
        format!("{label} · loading game data")
    }
}

#[component]
pub fn SetupRoutePage() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut metadata_resource = use_resource(move || async move {
        let snapshot = game.read().clone();
        if !snapshot.metadata_current() {
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

    let load_error = metadata_resource
        .value()
        .read()
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());
    rsx! { SetupPanel { game, load_error } }
}

#[component]
pub fn ReviewRoutePage() -> Element {
    let game = use_context::<Signal<Game>>();
    rsx! { ReviewPanel { game } }
}

#[component]
pub fn StudyIndexRoutePage() -> Element {
    let navigator = use_navigator();
    use_effect(move || {
        navigator.replace(Route::StudySet {
            set_id: "doctrinal-mastery-all".to_string(),
        });
    });
    rsx! {}
}

#[component]
pub fn StudySetRoutePage(set_id: String) -> Element {
    rsx! { StudyRouteContent { selected_id: set_id } }
}

#[component]
fn StudyRouteContent(selected_id: String) -> Element {
    let game = use_context::<Signal<Game>>();
    let metadata_resource = use_study_metadata(game);
    let load_error = metadata_resource
        .value()
        .read()
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());

    rsx! { StudySetsPanel { game, selected_id, load_error } }
}

#[component]
pub fn AtlasRoutePage() -> Element {
    let game = use_context::<Signal<Game>>();
    let metadata_resource = use_study_metadata(game);
    let load_error = metadata_resource
        .value()
        .read()
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());

    rsx! { AtlasPanel { game, load_error } }
}

fn use_study_metadata(
    mut game: Signal<Game>,
) -> Resource<Result<Option<crate::api::MetadataResponse>, String>> {
    let mut resource = use_resource(move || async move {
        let snapshot = game.read().clone();
        if snapshot.study_metadata.is_empty() {
            load_metadata(snapshot.study_metadata_request())
                .await
                .map(Some)
        } else {
            Ok(None)
        }
    });

    use_effect(move || {
        let Some(Ok(Some(metadata))) = resource.value().read().as_ref().cloned() else {
            return;
        };
        game.write().apply_study_metadata(metadata);
        resource.clear();
    });

    resource
}

#[component]
pub fn NotFoundRoutePage(segments: Vec<String>) -> Element {
    let navigator = use_navigator();
    use_effect(move || {
        navigator.replace(Route::Setup {});
    });
    rsx! {}
}

fn request_new_game(mut game: Signal<Game>) {
    let request = game.read().new_game_request();
    game.write().begin_starting_game();

    spawn(async move {
        match create_game(request).await {
            Ok(response) => {
                game.write().start_game(response);
                scroll_round_into_view_on_mobile();
            }
            Err(error) => game.write().fail_request(error),
        }
    });
}

fn request_review_game(mut game: Signal<Game>) {
    let Some(request) = game.read().review_game_request() else {
        return;
    };
    game.write().begin_starting_game();

    spawn(async move {
        match create_game(request).await {
            Ok(response) => {
                game.write()
                    .start_game_named(response, Some("Marked verses".to_string()));
                scroll_round_into_view_on_mobile();
            }
            Err(error) => game.write().fail_request(error),
        }
    });
}

fn request_study_game(mut game: Signal<Game>, set: StudySet, round_count: usize) {
    let Some(request) = game.read().study_set_game_request(&set, round_count) else {
        return;
    };
    let label = set.name;
    game.write().begin_starting_game();

    spawn(async move {
        match create_game(request).await {
            Ok(response) => {
                game.write().start_game_named(response, Some(label));
                scroll_round_into_view_on_mobile();
            }
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
            Ok(response) => {
                game.write().apply_guess(response);
                scroll_result_into_view_on_mobile();
            }
            Err(error) => game.write().fail_request(error),
        }
    });
}

#[component]
fn VersePanel(game: Signal<Game>) -> Element {
    let snapshot = game.read().clone();
    let round_number = snapshot.current_round_index + 1;
    let active = snapshot.active_game.as_ref();
    let mode_label = active
        .and_then(|game| game.label.clone())
        .unwrap_or_else(|| snapshot.settings.selection_label());
    let round_count = active
        .map(|game| game.round_count)
        .unwrap_or(snapshot.rounds.len());

    rsx! {
        section { class: "panel verse-panel",
            div { class: "verse-label",
                span { "Round {round_number} of {round_count}" }
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

#[cfg(target_arch = "wasm32")]
fn scroll_round_into_view_on_mobile() {
    let Some(window) = web_sys::window() else {
        return;
    };

    let callback = Closure::once(move || {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(document) = window.document() else {
            return;
        };
        let Some(element) = document.query_selector(".verse-panel").ok().flatten() else {
            return;
        };

        let options = web_sys::ScrollIntoViewOptions::new();
        options.set_behavior(web_sys::ScrollBehavior::Smooth);
        options.set_block(web_sys::ScrollLogicalPosition::Start);
        element.scroll_into_view_with_scroll_into_view_options(&options);
    });

    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        callback.as_ref().unchecked_ref(),
        0,
    );
    callback.forget();
}

#[cfg(not(target_arch = "wasm32"))]
fn scroll_round_into_view_on_mobile() {}

#[cfg(target_arch = "wasm32")]
fn scroll_result_into_view_on_mobile() {
    scroll_selector_into_view(".result", web_sys::ScrollLogicalPosition::Start);
}

#[cfg(not(target_arch = "wasm32"))]
fn scroll_result_into_view_on_mobile() {}

#[cfg(target_arch = "wasm32")]
fn scroll_answer_verse_into_view() {
    scroll_selector_into_view(
        ".chapter-dialog .answer-verse",
        web_sys::ScrollLogicalPosition::Center,
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn scroll_answer_verse_into_view() {}

#[cfg(target_arch = "wasm32")]
fn scroll_selector_into_view(selector: &'static str, block: web_sys::ScrollLogicalPosition) {
    let Some(window) = web_sys::window() else {
        return;
    };

    let callback = Closure::once(move || {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(document) = window.document() else {
            return;
        };
        let Some(element) = document.query_selector(selector).ok().flatten() else {
            return;
        };

        let options = web_sys::ScrollIntoViewOptions::new();
        options.set_behavior(web_sys::ScrollBehavior::Smooth);
        options.set_block(block);
        element.scroll_into_view_with_scroll_into_view_options(&options);
    });

    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        callback.as_ref().unchecked_ref(),
        0,
    );
    callback.forget();
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
    let navigator = use_navigator();
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
                onclick: move |_| {
                    game.write().change_settings();
                    navigator.push(Route::Setup {});
                },
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
    let canon_count = snapshot.guess_scope().canons.len();
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
                    for canon in snapshot.guess_scope().canons.iter().map(|scope| scope.canon) {
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
                    div { class: "grid chapter-grid chapter-matrix",
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
                if snapshot.is_last_round() {
                    button {
                        class: "button",
                        onclick: move |_| game.write().next_round(),
                        "Finish game"
                    }
                } else {
                    button {
                        class: "button",
                        onclick: move |_| {
                            game.write().next_round();
                            scroll_round_into_view_on_mobile();
                        },
                        "Next round"
                    }
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
fn ResultPanel(game: Signal<Game>, result: GuessResult) -> Element {
    let mut reader_open = use_signal(|| false);
    let same_canon = result.answer.canon == result.guess.canon;
    let label = if same_canon {
        result.score.distance_label()
    } else {
        "Wrong canon".to_string()
    };
    let chapter_title = format!("{} {}", result.answer.book, result.answer.chapter);
    let score_percent = result.score.points.saturating_mul(100) / MAX_SCORE;
    let marked_for_review = game.read().current_result_marked_for_review();
    let (feedback_title, feedback_detail) = result_feedback(&result);
    let answer_label = result.answer.label();
    let source_label = result.source_passage.label();

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
                    strong { "{answer_label}" }
                }
                div {
                    span { class: "muted", "Guess" }
                    strong { "{result.guess.book} {result.guess.chapter}" }
                }
            }
            if result.source_passage != result.answer {
                div { class: "ready result-source",
                    span { class: "muted", "Study passage" }
                    strong { "{source_label}" }
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
                    answer: result.answer.clone(),
                    verses: result.chapter_verses.clone(),
                    on_close: move |_| reader_open.set(false),
                }
            }
        }
    }
}

fn result_feedback(result: &GuessResult) -> (&'static str, String) {
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

#[component]
fn ChapterReader(
    title: String,
    answer: StudyPassage,
    verses: Vec<crate::api::ChapterVerse>,
    on_close: EventHandler<MouseEvent>,
) -> Element {
    use_effect(move || {
        scroll_answer_verse_into_view();
    });

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
                            class: if answer.contains_verse(verse.verse) { "chapter-verse answer-verse" } else { "chapter-verse" },
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
                            "{guess.answer.label()}"
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
