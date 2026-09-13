use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

use crate::scoring::{MAX_SCORE, Score};
use crate::scriptures::{Difficulty, Scriptures, Verse};
use crate::stats::{FinishedGame, FinishedRound, Stats};

#[derive(Clone)]
pub struct Game {
    pub scriptures: Scriptures,
    pub settings: GameSettings,
    pub stats: Stats,
    pub last_game_new_best: bool,
    pub screen: Screen,
    pub rounds: Vec<Round>,
    pub current_round_index: usize,
    pub selected_canon: bool,
    pub selected_book: Option<String>,
    pub selected_chapter: Option<u16>,
    pub active_step: GuessStep,
    pub finished: bool,
    rng: SmallRng,
}

impl PartialEq for Game {
    fn eq(&self, other: &Self) -> bool {
        self.scriptures == other.scriptures
            && self.settings == other.settings
            && self.stats == other.stats
            && self.last_game_new_best == other.last_game_new_best
            && self.screen == other.screen
            && self.rounds == other.rounds
            && self.current_round_index == other.current_round_index
            && self.selected_canon == other.selected_canon
            && self.selected_book == other.selected_book
            && self.selected_chapter == other.selected_chapter
            && self.active_step == other.active_step
            && self.finished == other.finished
    }
}

impl Game {
    pub fn new(scriptures: Scriptures) -> Self {
        Self {
            scriptures,
            settings: GameSettings::default(),
            stats: Stats::load(),
            last_game_new_best: false,
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

    pub fn start_game(&mut self) {
        self.screen = Screen::Playing;
        self.restart();
    }

    pub fn restart(&mut self) {
        self.last_game_new_best = false;
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

    pub fn change_settings(&mut self) {
        self.screen = Screen::Setup;
        self.finished = false;
        self.last_game_new_best = false;
        self.rounds.clear();
        self.current_round_index = 0;
        self.clear_guess();
    }

    pub fn set_round_count(&mut self, round_count: usize) {
        self.settings.round_count = round_count;
    }

    pub fn set_difficulty(&mut self, difficulty: Difficulty) {
        self.settings.difficulty = difficulty;
    }

    pub fn current_round(&self) -> &Round {
        &self.rounds[self.current_round_index]
    }

    pub fn is_last_round(&self) -> bool {
        self.current_round_index + 1 == self.rounds.len()
    }

    pub fn max_total_score(&self) -> u32 {
        self.settings.round_count as u32 * MAX_SCORE
    }

    pub fn select_canon(&mut self) {
        self.selected_canon = true;
        self.active_step = GuessStep::Book;
    }

    pub fn select_book(&mut self, book: String) {
        if self.selected_book.as_ref() != Some(&book) {
            self.selected_chapter = None;
        }
        self.selected_book = Some(book);
        self.active_step = GuessStep::Chapter;
    }

    pub fn select_chapter(&mut self, chapter: u16) {
        self.selected_chapter = Some(chapter);
        self.active_step = GuessStep::Ready;
    }

    pub fn open_canon(&mut self) {
        if self.selected_canon {
            self.active_step = GuessStep::Book;
        }
    }

    pub fn open_book(&mut self) {
        if self.selected_book.is_some() {
            self.active_step = GuessStep::Chapter;
        }
    }

    pub fn open_chapter(&mut self) {
        if self.selected_chapter.is_some() {
            self.active_step = GuessStep::Ready;
        }
    }

    pub fn clear_guess(&mut self) {
        self.selected_canon = false;
        self.selected_book = None;
        self.selected_chapter = None;
        self.active_step = GuessStep::Canon;
    }

    pub fn submit_guess(&mut self) {
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

    pub fn next_round(&mut self) {
        if self.finished {
            return;
        }

        if self.is_last_round() {
            self.finished = true;
            self.record_finished_game();
        } else {
            self.current_round_index += 1;
            self.selected_canon = false;
            self.selected_book = None;
            self.selected_chapter = None;
            self.active_step = GuessStep::Canon;
        }
    }

    pub fn total_score(&self) -> u32 {
        self.rounds
            .iter()
            .filter_map(|round| round.guess.as_ref().map(|guess| guess.score.points))
            .sum()
    }

    pub fn chapters_for(&self, book: &str) -> Vec<u16> {
        self.scriptures.chapters_for(book)
    }

    fn record_finished_game(&mut self) {
        let finished_game = FinishedGame {
            difficulty: self.settings.difficulty,
            score: self.total_score(),
            possible_score: self.max_total_score(),
            rounds: self
                .rounds
                .iter()
                .filter_map(|round| {
                    let guess = round.guess.as_ref()?;
                    Some(FinishedRound {
                        answer_book: round.verse.reference.book.clone(),
                        score: guess.score.points,
                        possible_score: MAX_SCORE,
                    })
                })
                .collect(),
        };

        self.last_game_new_best = self.stats.record_game(finished_game);
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct GameSettings {
    pub round_count: usize,
    pub difficulty: Difficulty,
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
pub enum Screen {
    Setup,
    Playing,
}

#[derive(Clone, Copy, PartialEq)]
pub enum GuessStep {
    Canon,
    Book,
    Chapter,
    Ready,
}

impl GuessStep {
    pub fn label(self) -> &'static str {
        match self {
            GuessStep::Canon => "Canon",
            GuessStep::Book => "Book",
            GuessStep::Chapter => "Chapter",
            GuessStep::Ready => "Review",
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct Round {
    pub verse: Verse,
    pub guess: Option<GuessResult>,
}

#[derive(Clone, PartialEq)]
pub struct GuessResult {
    pub book: String,
    pub chapter: u16,
    pub score: Score,
}
