use dioxus::prelude::*;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use serde::Deserialize;

const DATA: &str = include_str!("../data/book-of-mormon-flat.json");
const ROUNDS_PER_GAME: usize = 5;
const MAX_SCORE: u32 = 1000;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let game = use_signal(|| Game::new(load_scriptures()));
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
                        span { "Book of Mormon · 5 rounds · book and chapter" }
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
                div { class: "actions",
                    button {
                        class: "button",
                        onclick: move |_| game.write().restart(),
                        "Play again"
                    }
                }
                RoundSummary { rounds: snapshot.rounds.clone() }
            } else {
                blockquote { class: "verse", "{snapshot.current_round().verse.text}" }

                if snapshot.current_round().guess.is_some() {
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
fn GuessPanel(game: Signal<Game>) -> Element {
    let snapshot = game.read().clone();
    let selected_book = snapshot.selected_book.clone();
    let selected_chapter = snapshot.selected_chapter;
    let guessed = snapshot.current_round().guess.is_some();

    rsx! {
        aside { class: "panel picker",
            div { class: "picker-header",
                h2 { "Guess location" }
                span { class: "muted", "Canon" }
            }

            div { class: "grid",
                button { class: "choice active", "Book of Mormon" }
            }

            div { class: "picker-header", style: "margin-top: 18px;",
                h2 { "Book" }
            }
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

            if let Some(book_name) = selected_book.clone() {
                div { class: "picker-header", style: "margin-top: 18px;",
                    h2 { "Chapter" }
                    span { class: "muted", "{book_name}" }
                }
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

            p { class: "selected",
                "Selected: "
                if let (Some(book), Some(chapter)) = (selected_book.clone(), selected_chapter) {
                    strong { "{book} {chapter}" }
                } else {
                    span { "choose a book and chapter" }
                }
            }

            div { class: "actions",
                button {
                    class: "button",
                    disabled: guessed || selected_book.is_none() || selected_chapter.is_none(),
                    onclick: move |_| game.write().submit_guess(),
                    "Submit guess"
                }
            }

            if let Some(result) = snapshot.current_round().guess.clone() {
                ResultPanel { result: result, answer: snapshot.current_round().verse.reference.clone() }
            }
        }
    }
}

#[component]
fn ResultPanel(result: GuessResult, answer: Reference) -> Element {
    rsx! {
        div { class: "result",
            h2 { "Result" }
            p { class: "score", "{result.score}" }
            p { "Actual: " strong { "{answer.book} {answer.chapter}" } }
            p { "Guess: " strong { "{result.book} {result.chapter}" } }
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
                    span { "{round.guess.as_ref().map(|guess| guess.score).unwrap_or_default()} pts" }
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
    selected_book: Option<String>,
    selected_chapter: Option<u16>,
    finished: bool,
    rng: SmallRng,
}

impl Game {
    fn new(scriptures: Scriptures) -> Self {
        let mut game = Self {
            scriptures,
            rounds: Vec::new(),
            current_round_index: 0,
            selected_book: None,
            selected_chapter: None,
            finished: false,
            rng: SmallRng::from_os_rng(),
        };
        game.restart();
        game
    }

    fn restart(&mut self) {
        self.rounds = (0..ROUNDS_PER_GAME)
            .map(|_| {
                let index = self.rng.random_range(0..self.scriptures.verses.len());
                Round {
                    verse: self.scriptures.verses[index].clone(),
                    guess: None,
                }
            })
            .collect();
        self.current_round_index = 0;
        self.selected_book = None;
        self.selected_chapter = None;
        self.finished = false;
    }

    fn current_round(&self) -> &Round {
        &self.rounds[self.current_round_index]
    }

    fn is_last_round(&self) -> bool {
        self.current_round_index + 1 == self.rounds.len()
    }

    fn select_book(&mut self, book: String) {
        self.selected_book = Some(book);
        self.selected_chapter = None;
    }

    fn select_chapter(&mut self, chapter: u16) {
        self.selected_chapter = Some(chapter);
    }

    fn submit_guess(&mut self) {
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
            self.selected_book = None;
            self.selected_chapter = None;
        }
    }

    fn total_score(&self) -> u32 {
        self.rounds
            .iter()
            .filter_map(|round| round.guess.as_ref().map(|guess| guess.score))
            .sum()
    }

    fn books(&self) -> Vec<BookInfo> {
        self.scriptures.books.clone()
    }

    fn chapters_for(&self, book: &str) -> Vec<u16> {
        self.scriptures
            .books
            .iter()
            .find(|item| item.name == book)
            .map(|item| item.chapters.clone())
            .unwrap_or_default()
    }
}

#[derive(Clone)]
struct Scriptures {
    verses: Vec<Verse>,
    books: Vec<BookInfo>,
    chapter_order: Vec<ChapterRef>,
}

impl Scriptures {
    fn score(&self, answer: &Reference, guess_book: &str, guess_chapter: u16) -> u32 {
        let Some(answer_index) = self.chapter_index(&answer.book, answer.chapter) else {
            return 0;
        };
        let Some(guess_index) = self.chapter_index(guess_book, guess_chapter) else {
            return 0;
        };
        let distance = answer_index.abs_diff(guess_index) as u32;

        if distance == 0 {
            MAX_SCORE
        } else {
            MAX_SCORE.saturating_sub(distance * 22)
        }
    }

    fn chapter_index(&self, book: &str, chapter: u16) -> Option<usize> {
        self.chapter_order
            .iter()
            .position(|item| item.book == book && item.chapter == chapter)
    }
}

#[derive(Clone, PartialEq)]
struct BookInfo {
    name: String,
    chapters: Vec<u16>,
}

#[derive(Clone, PartialEq)]
struct ChapterRef {
    book: String,
    chapter: u16,
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
    score: u32,
}

#[derive(Clone, PartialEq)]
struct Verse {
    reference: Reference,
    text: String,
}

#[derive(Clone, PartialEq)]
struct Reference {
    book: String,
    chapter: u16,
}

#[derive(Deserialize)]
struct FlatScriptures {
    verses: Vec<FlatVerse>,
}

#[derive(Deserialize)]
struct FlatVerse {
    reference: String,
    text: String,
}

fn load_scriptures() -> Scriptures {
    let flat: FlatScriptures = serde_json::from_str(DATA).expect("bundled scripture data is valid");
    let mut verses = Vec::new();
    let mut books = Vec::<BookInfo>::new();
    let mut chapter_order = Vec::<ChapterRef>::new();

    for item in flat.verses {
        let Some(reference) = parse_reference(&item.reference) else {
            continue;
        };

        if books.last().map(|book| &book.name) != Some(&reference.book) {
            books.push(BookInfo {
                name: reference.book.clone(),
                chapters: Vec::new(),
            });
        }

        let book = books.last_mut().expect("book was just inserted if missing");
        if book.chapters.last() != Some(&reference.chapter) {
            book.chapters.push(reference.chapter);
            chapter_order.push(ChapterRef {
                book: reference.book.clone(),
                chapter: reference.chapter,
            });
        }

        verses.push(Verse {
            reference,
            text: item.text,
        });
    }

    Scriptures {
        verses,
        books,
        chapter_order,
    }
}

fn parse_reference(reference: &str) -> Option<Reference> {
    let (book_and_chapter, _verse) = reference.rsplit_once(':')?;
    let last_space = book_and_chapter.rfind(' ')?;
    let (book, chapter) = book_and_chapter.split_at(last_space);
    let chapter = chapter.trim().parse().ok()?;

    Some(Reference {
        book: book.to_string(),
        chapter,
    })
}
