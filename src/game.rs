use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

use crate::scoring::{MAX_SCORE, Score};
use crate::scriptures::{Canon, Difficulty, GameMode, ScriptureLibrary, Scriptures, Verse};
use crate::stats::{FinishedGame, FinishedRound, Stats};

#[derive(Clone)]
pub struct Game {
    pub library: ScriptureLibrary,
    pub settings: GameSettings,
    pub stats: Stats,
    pub last_game_new_best: bool,
    pub screen: Screen,
    pub rounds: Vec<Round>,
    pub current_round_index: usize,
    pub selected_canon: Option<Canon>,
    pub selected_book: Option<String>,
    pub selected_chapter: Option<u16>,
    pub active_step: GuessStep,
    pub finished: bool,
    rng: SmallRng,
}

impl PartialEq for Game {
    fn eq(&self, other: &Self) -> bool {
        self.settings == other.settings
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
    pub fn new() -> Self {
        Self {
            library: ScriptureLibrary::default(),
            settings: GameSettings::default(),
            stats: Stats::load(),
            last_game_new_best: false,
            screen: Screen::Setup,
            rounds: Vec::new(),
            current_round_index: 0,
            selected_canon: None,
            selected_book: None,
            selected_chapter: None,
            active_step: GuessStep::Canon,
            finished: false,
            rng: SmallRng::from_os_rng(),
        }
    }

    pub fn start_game(&mut self) {
        if !self.selected_canons_loaded() {
            return;
        }
        self.screen = Screen::Playing;
        self.restart();
    }

    pub fn restart(&mut self) {
        self.last_game_new_best = false;
        self.rounds = (0..self.settings.round_count)
            .map(|_| {
                let mut index = self.rng.random_range(0..self.verse_count_for_difficulty());
                let mut verse = None;

                for canon in &self.settings.canons {
                    let pool = self
                        .scriptures_for(*canon)
                        .verses_for_difficulty(self.settings.difficulty);

                    if index < pool.len() {
                        verse = Some(pool[index].clone());
                        break;
                    }

                    index -= pool.len();
                }

                Round {
                    verse: verse.expect("loaded game mode has at least one playable verse"),
                    guess: None,
                }
            })
            .collect();
        self.current_round_index = 0;
        self.reset_guess_path();
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

    pub fn set_preset(&mut self, mode: GameMode) {
        let canons = mode.canons().to_vec();
        if self.settings.canons != canons {
            self.settings.canons = canons;
            self.clear_guess();
        }
    }

    pub fn toggle_canon(&mut self, canon: Canon) {
        if self.settings.canons.contains(&canon) {
            if self.settings.canons.len() == 1 {
                return;
            }

            self.settings.canons.retain(|selected| *selected != canon);
        } else {
            self.settings.canons.push(canon);
            self.settings
                .canons
                .sort_by_key(|canon| Canon::ALL.iter().position(|item| item == canon));
        }

        self.clear_guess();
    }

    pub fn set_scriptures(&mut self, canon: Canon, scriptures: Scriptures) {
        self.library.insert(canon, scriptures);
    }

    pub fn selected_canons_loaded(&self) -> bool {
        self.settings
            .canons
            .iter()
            .all(|canon| self.library.has_canon(*canon))
    }

    pub fn first_unloaded_canon(&self) -> Option<Canon> {
        self.settings
            .canons
            .iter()
            .copied()
            .find(|canon| !self.library.has_canon(*canon))
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

    pub fn select_canon(&mut self, canon: Canon) {
        if self.selected_canon != Some(canon) {
            self.selected_book = None;
            self.selected_chapter = None;
        }
        self.selected_canon = Some(canon);
        self.active_step = GuessStep::Book;
        self.advance_past_single_option_steps();
    }

    pub fn select_book(&mut self, book: String) {
        if self.selected_book.as_ref() != Some(&book) {
            self.selected_chapter = None;
        }
        self.selected_book = Some(book);
        self.active_step = GuessStep::Chapter;
        self.advance_past_single_option_steps();
    }

    pub fn select_chapter(&mut self, chapter: u16) {
        self.selected_chapter = Some(chapter);
        self.active_step = GuessStep::Ready;
    }

    pub fn open_canon(&mut self) {
        if self.selected_canon.is_some() {
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
        self.reset_guess_path();
    }

    fn reset_guess_path(&mut self) {
        self.selected_canon = None;
        self.selected_book = None;
        self.selected_chapter = None;
        self.active_step = GuessStep::Canon;
        self.advance_past_single_option_steps();
    }

    fn advance_past_single_option_steps(&mut self) {
        if !self.selected_canons_loaded() {
            return;
        }

        if self.active_step == GuessStep::Canon && self.settings.canons.len() == 1 {
            self.selected_canon = self.settings.canons.first().copied();
            self.active_step = GuessStep::Book;
        }

        if self.active_step == GuessStep::Book {
            let Some(canon) = self.selected_canon else {
                return;
            };
            let books = &self.scriptures_for(canon).books;
            if books.len() == 1 {
                self.selected_book = Some(books[0].name.clone());
                self.active_step = GuessStep::Chapter;
            }
        }

        if self.active_step == GuessStep::Chapter {
            let Some(canon) = self.selected_canon else {
                return;
            };
            let Some(book) = self.selected_book.as_deref() else {
                return;
            };
            let chapters = self.chapters_for(canon, book);
            if chapters.len() == 1 {
                self.selected_chapter = chapters.first().copied();
                self.active_step = GuessStep::Ready;
            }
        }
    }

    pub fn submit_guess(&mut self) {
        if self.selected_canon.is_none() {
            return;
        }
        let Some(book) = self.selected_book.clone() else {
            return;
        };
        let Some(chapter) = self.selected_chapter else {
            return;
        };
        let Some(canon) = self.selected_canon else {
            return;
        };
        let answer = self.current_round().verse.reference.clone();
        let score = self
            .library
            .score(&self.settings.canons, &answer, canon, &book, chapter);
        self.rounds[self.current_round_index].guess = Some(GuessResult {
            canon,
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
            self.reset_guess_path();
        }
    }

    pub fn total_score(&self) -> u32 {
        self.rounds
            .iter()
            .filter_map(|round| round.guess.as_ref().map(|guess| guess.score.points))
            .sum()
    }

    pub fn chapters_for(&self, canon: Canon, book: &str) -> Vec<u16> {
        self.scriptures_for(canon).chapters_for(book)
    }

    pub fn scriptures_for(&self, canon: Canon) -> &Scriptures {
        self.library
            .scriptures(canon)
            .expect("selected canon is loaded before gameplay starts")
    }

    pub fn total_verse_count(&self) -> usize {
        self.settings
            .canons
            .iter()
            .filter_map(|canon| self.library.scriptures(*canon))
            .map(Scriptures::total_verse_count)
            .sum()
    }

    pub fn verse_count_for_difficulty(&self) -> usize {
        self.settings
            .canons
            .iter()
            .filter_map(|canon| self.library.scriptures(*canon))
            .map(|scriptures| scriptures.verse_count_for_difficulty(self.settings.difficulty))
            .sum()
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

#[derive(Clone, PartialEq)]
pub struct GameSettings {
    pub round_count: usize,
    pub difficulty: Difficulty,
    pub canons: Vec<Canon>,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            round_count: 5,
            difficulty: Difficulty::Normal,
            canons: GameMode::BookOfMormon.canons().to_vec(),
        }
    }
}

impl GameSettings {
    pub fn selection_label(&self) -> &'static str {
        GameMode::ALL
            .iter()
            .copied()
            .find(|mode| mode.canons() == self.canons.as_slice())
            .map(GameMode::label)
            .unwrap_or("Custom")
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
    pub canon: Canon,
    pub book: String,
    pub chapter: u16,
    pub score: Score,
}
