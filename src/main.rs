use dioxus::prelude::*;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

mod scoring;
mod scriptures;

use scoring::{MAX_SCORE, Score};
use scriptures::{BookInfo, Reference, Scriptures, Verse};

const DATA: &str = include_str!("../data/book-of-mormon-flat.json");
const ROUNDS_PER_GAME: usize = 5;

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
                        span { "Book of Mormon · 5 rounds · {snapshot.scriptures.playable_verses.len()} of {snapshot.scriptures.total_verse_count()} verses in play" }
                    }
                    div { class: "pill", "Total {snapshot.total_score()} / {ROUNDS_PER_GAME as u32 * MAX_SCORE}" }
                }

                div { class: "layout",
                    VersePanel { game: game }
                    GuessPanel { game: game }
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
                span { "Round {round_number} of {ROUNDS_PER_GAME}" }
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
                    strong { "{snapshot.total_score()} / {ROUNDS_PER_GAME as u32 * MAX_SCORE}" }
                }
                div { class: "actions",
                    button {
                        class: "button",
                        onclick: move |_| game.write().restart(),
                        "Play again"
                    }
                }
            } else {
                div { class: "picker-header",
                    h2 { "Guess location" }
                    span { class: "muted", "{step.label()}" }
                }

                if selected_canon {
                    nav { class: "breadcrumbs", aria_label: "Guess path",
                        button {
                            class: if step == GuessStep::Canon { "crumb current" } else { "crumb set" },
                            disabled: guessed,
                            onclick: move |_| game.write().edit_canon(),
                            "Book of Mormon"
                        }
                        if let Some(book) = selected_book.clone() {
                            span { class: "crumb-separator", "/" }
                            button {
                                class: if step == GuessStep::Book { "crumb current" } else { "crumb set" },
                                disabled: guessed,
                                onclick: move |_| game.write().edit_book(),
                                "{book}"
                            }
                        }
                        if let Some(chapter) = selected_chapter {
                            span { class: "crumb-separator", "/" }
                            button {
                                class: if step == GuessStep::Chapter { "crumb current" } else { "crumb set" },
                                disabled: guessed,
                                onclick: move |_| game.write().edit_chapter(),
                                "Chapter {chapter}"
                            }
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
        let mut game = Self {
            scriptures,
            rounds: Vec::new(),
            current_round_index: 0,
            selected_canon: false,
            selected_book: None,
            selected_chapter: None,
            active_step: GuessStep::Canon,
            finished: false,
            rng: SmallRng::from_os_rng(),
        };
        game.restart();
        game
    }

    fn restart(&mut self) {
        self.rounds = (0..ROUNDS_PER_GAME)
            .map(|_| {
                let verse_pool = &self.scriptures.playable_verses;
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

    fn current_round(&self) -> &Round {
        &self.rounds[self.current_round_index]
    }

    fn is_last_round(&self) -> bool {
        self.current_round_index + 1 == self.rounds.len()
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

    fn edit_canon(&mut self) {
        self.active_step = GuessStep::Canon;
    }

    fn edit_book(&mut self) {
        if self.selected_canon {
            self.active_step = GuessStep::Book;
        }
    }

    fn edit_chapter(&mut self) {
        if self.selected_book.is_some() {
            self.active_step = GuessStep::Chapter;
        }
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
