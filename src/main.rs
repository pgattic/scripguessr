use dioxus::prelude::*;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

mod scoring;
mod scriptures;

use scoring::{MAX_SCORE, Score};
use scriptures::{BookInfo, Difficulty, Reference, Scriptures, Verse};

const DATA: &str = include_str!("../data/book-of-mormon-flat.json");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let game = use_signal(|| {
        Game::new(Scriptures::from_flat_json(DATA).expect("bundled scripture data is valid"))
    });
    let snapshot = game.read().clone();

    rsx! {
        document::Stylesheet {
            href: asset!("/assets/main.css")
        }
        main { class: "app",
            div { class: "shell",
                header { class: "topbar",
                    div { class: "brand",
                        h1 { "ScripGuessr" }
                        span { "Book of Mormon · {snapshot.settings.round_count} rounds · {snapshot.settings.difficulty.label()} · {snapshot.scriptures.verse_count_for_difficulty(snapshot.settings.difficulty)} of {snapshot.scriptures.total_verse_count()} verses in play" }
                    }
                    if snapshot.screen == Screen::Playing {
                        div { class: "pill", "Total {snapshot.total_score()} / {snapshot.max_total_score()}" }
                    }
                }

                if snapshot.screen == Screen::Setup {
                    SetupPanel { game: game }
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

    rsx! {
        section { class: "panel verse-panel",
            div { class: "verse-label",
                span { "Round {round_number} of {snapshot.settings.round_count}" }
                span { "Book of Mormon" }
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
    let selected_canon = snapshot.selected_canon;
    let selected_book = snapshot.selected_book.clone();
    let selected_chapter = snapshot.selected_chapter;
    let guessed = !snapshot.finished && snapshot.current_round().guess.is_some();
    let step = snapshot.active_step;
    let can_submit = selected_canon && selected_book.is_some() && selected_chapter.is_some();

    rsx! {
        aside { class: "panel picker",
            if snapshot.finished {
                div { class: "picker-header",
                    h2 { "Game complete" }
                    span { class: "muted", "{snapshot.total_score()} pts" }
                }
                div { class: "ready",
                    span { class: "muted", "Final score" }
                    strong { "{snapshot.total_score()} / {snapshot.max_total_score()}" }
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
            } else {
                div { class: "picker-header",
                    h2 { "Guess location" }
                    span { class: "muted", "{step.label()}" }
                }

                if selected_canon {
                    div { class: "breadcrumb-bar",
                        nav { class: "breadcrumbs", aria_label: "Guess path",
                            button {
                                class: if step == GuessStep::Book { "crumb current" } else { "crumb set" },
                                disabled: guessed,
                                onclick: move |_| game.write().open_canon(),
                                "Book of Mormon"
                            }
                            if let Some(book) = selected_book.clone() {
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

                match step {
                    GuessStep::Canon => rsx! {
                        div { class: "grid",
                            button {
                                class: if selected_canon { "choice active" } else { "choice" },
                                disabled: guessed,
                                onclick: move |_| game.write().select_canon(),
                                "Book of Mormon"
                            }
                        }
                    },
                    GuessStep::Book => rsx! {
                        div { class: "grid book-grid",
                            for book in snapshot.books() {
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
                    ResultPanel { result: result, answer: snapshot.current_round().verse.reference.clone() }
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
    }
}

#[component]
fn SetupPanel(game: Signal<Game>) -> Element {
    let snapshot = game.read().clone();

    rsx! {
        section { class: "panel setup-panel",
            div { class: "picker-header",
                h2 { "New game" }
                span { class: "muted", "Book of Mormon" }
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
                    button { class: "choice active", "Book of Mormon" }
                }
            }

            div { class: "ready",
                span { class: "muted", "Verse pool" }
                strong { "{snapshot.scriptures.verse_count_for_difficulty(snapshot.settings.difficulty)} of {snapshot.scriptures.total_verse_count()} verses" }
            }

            div { class: "actions",
                button {
                    class: "button",
                    onclick: move |_| game.write().start_game(),
                    "Start"
                }
            }
        }
    }
}

#[component]
fn ResultPanel(result: GuessResult, answer: Reference) -> Element {
    let label = result.score.distance_label();

    rsx! {
        div { class: "result",
            h2 { "Result" }
            p { class: "score", "{result.score.points}" }
            p { class: "distance", "{label}" }
            div { class: "result-grid",
                div {
                    span { class: "muted", "Actual" }
                    strong { "{answer.book} {answer.chapter}" }
                }
                div {
                    span { class: "muted", "Guess" }
                    strong { "{result.book} {result.chapter}" }
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
                    span { "{round.verse.reference.book} {round.verse.reference.chapter}" }
                    span { "{round.guess.as_ref().map(|guess| guess.score.points).unwrap_or_default()} pts" }
                }
            }
        }
    }
}

#[derive(Clone)]
struct Game {
    scriptures: Scriptures,
    settings: GameSettings,
    screen: Screen,
    rounds: Vec<Round>,
    current_round_index: usize,
    selected_canon: bool,
    selected_book: Option<String>,
    selected_chapter: Option<u16>,
    active_step: GuessStep,
    finished: bool,
    rng: SmallRng,
}

impl Game {
    fn new(scriptures: Scriptures) -> Self {
        Self {
            scriptures,
            settings: GameSettings::default(),
            screen: Screen::Setup,
            rounds: Vec::new(),
            current_round_index: 0,
            selected_canon: false,
            selected_book: None,
            selected_chapter: None,
            active_step: GuessStep::Canon,
            finished: false,
            rng: SmallRng::from_os_rng(),
        }
    }

    fn start_game(&mut self) {
        self.screen = Screen::Playing;
        self.restart();
    }

    fn restart(&mut self) {
        self.rounds = (0..self.settings.round_count)
            .map(|_| {
                let verse_pool = self
                    .scriptures
                    .verses_for_difficulty(self.settings.difficulty);
                let index = self.rng.random_range(0..verse_pool.len());
                Round {
                    verse: verse_pool[index].clone(),
                    guess: None,
                }
            })
            .collect();
        self.current_round_index = 0;
        self.selected_canon = false;
        self.selected_book = None;
        self.selected_chapter = None;
        self.active_step = GuessStep::Canon;
        self.finished = false;
    }

    fn change_settings(&mut self) {
        self.screen = Screen::Setup;
        self.finished = false;
        self.rounds.clear();
        self.current_round_index = 0;
        self.clear_guess();
    }

    fn set_round_count(&mut self, round_count: usize) {
        self.settings.round_count = round_count;
    }

    fn set_difficulty(&mut self, difficulty: Difficulty) {
        self.settings.difficulty = difficulty;
    }

    fn current_round(&self) -> &Round {
        &self.rounds[self.current_round_index]
    }

    fn is_last_round(&self) -> bool {
        self.current_round_index + 1 == self.rounds.len()
    }

    fn max_total_score(&self) -> u32 {
        self.settings.round_count as u32 * MAX_SCORE
    }

    fn select_canon(&mut self) {
        self.selected_canon = true;
        self.active_step = GuessStep::Book;
    }

    fn select_book(&mut self, book: String) {
        if self.selected_book.as_ref() != Some(&book) {
            self.selected_chapter = None;
        }
        self.selected_book = Some(book);
        self.active_step = GuessStep::Chapter;
    }

    fn select_chapter(&mut self, chapter: u16) {
        self.selected_chapter = Some(chapter);
        self.active_step = GuessStep::Ready;
    }

    fn open_canon(&mut self) {
        if self.selected_canon {
            self.active_step = GuessStep::Book;
        }
    }

    fn open_book(&mut self) {
        if self.selected_book.is_some() {
            self.active_step = GuessStep::Chapter;
        }
    }

    fn open_chapter(&mut self) {
        if self.selected_chapter.is_some() {
            self.active_step = GuessStep::Ready;
        }
    }

    fn clear_guess(&mut self) {
        self.selected_canon = false;
        self.selected_book = None;
        self.selected_chapter = None;
        self.active_step = GuessStep::Canon;
    }

    fn submit_guess(&mut self) {
        if !self.selected_canon {
            return;
        }
        let Some(book) = self.selected_book.clone() else {
            return;
        };
        let Some(chapter) = self.selected_chapter else {
            return;
        };
        let answer = self.current_round().verse.reference.clone();
        let score = self.scriptures.score(&answer, &book, chapter);
        self.rounds[self.current_round_index].guess = Some(GuessResult {
            book,
            chapter,
            score,
        });
    }

    fn next_round(&mut self) {
        if self.is_last_round() {
            self.finished = true;
        } else {
            self.current_round_index += 1;
            self.selected_canon = false;
            self.selected_book = None;
            self.selected_chapter = None;
            self.active_step = GuessStep::Canon;
        }
    }

    fn total_score(&self) -> u32 {
        self.rounds
            .iter()
            .filter_map(|round| round.guess.as_ref().map(|guess| guess.score.points))
            .sum()
    }

    fn books(&self) -> Vec<BookInfo> {
        self.scriptures.books.clone()
    }

    fn chapters_for(&self, book: &str) -> Vec<u16> {
        self.scriptures.chapters_for(book)
    }
}

#[derive(Clone, Copy, PartialEq)]
struct GameSettings {
    round_count: usize,
    difficulty: Difficulty,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            round_count: 5,
            difficulty: Difficulty::Normal,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Setup,
    Playing,
}

#[derive(Clone, Copy, PartialEq)]
enum GuessStep {
    Canon,
    Book,
    Chapter,
    Ready,
}

impl GuessStep {
    fn label(self) -> &'static str {
        match self {
            GuessStep::Canon => "Canon",
            GuessStep::Book => "Book",
            GuessStep::Chapter => "Chapter",
            GuessStep::Ready => "Review",
        }
    }
}

#[derive(Clone, PartialEq)]
struct Round {
    verse: Verse,
    guess: Option<GuessResult>,
}

#[derive(Clone, PartialEq)]
struct GuessResult {
    book: String,
    chapter: u16,
    score: Score,
}
