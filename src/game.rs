use crate::api::{
    CanonMetadata, ChapterVerse, GuessReference, GuessRequest, GuessResponse, MetadataRequest,
    MetadataResponse, NewGameRequest, NewGameResponse,
};
use crate::scoring::{MAX_SCORE, Score};
use crate::scriptures::{BookInfo, Canon, Difficulty, GameMode, Reference};
use crate::stats::{FinishedGame, FinishedRound, ReviewItem, Stats};

#[derive(Clone, PartialEq)]
pub struct Game {
    pub game_id: Option<String>,
    pub metadata: Vec<CanonMetadata>,
    pub metadata_difficulty: Difficulty,
    pub playable_verse_count: usize,
    pub total_verse_count: usize,
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
    pub loading_game: bool,
    pub submitting_guess: bool,
    pub error: Option<String>,
}

impl Game {
    pub fn new() -> Self {
        Self {
            game_id: None,
            metadata: Vec::new(),
            metadata_difficulty: Difficulty::Normal,
            playable_verse_count: 0,
            total_verse_count: 0,
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
            loading_game: false,
            submitting_guess: false,
            error: None,
        }
    }

    pub fn metadata_request(&self) -> MetadataRequest {
        MetadataRequest {
            difficulty: self.settings.difficulty,
            canons: self.settings.canons.clone(),
        }
    }

    pub fn new_game_request(&self) -> NewGameRequest {
        NewGameRequest {
            round_count: self.settings.round_count,
            difficulty: self.settings.difficulty,
            canons: self.settings.canons.clone(),
        }
    }

    pub fn apply_metadata(&mut self, response: MetadataResponse) {
        if response.difficulty != self.settings.difficulty
            || response.canons != self.settings.canons
        {
            return;
        }

        self.metadata_difficulty = response.difficulty;
        self.metadata = response.metadata;
        self.playable_verse_count = response.playable_verse_count;
        self.total_verse_count = response.total_verse_count;
        self.advance_past_single_option_steps();
    }

    pub fn metadata_ready(&self) -> bool {
        self.metadata_difficulty == self.settings.difficulty
            && self
                .metadata
                .iter()
                .map(|item| item.canon)
                .collect::<Vec<_>>()
                == self.settings.canons
            && self.playable_verse_count > 0
    }

    pub fn begin_starting_game(&mut self) {
        self.loading_game = true;
        self.error = None;
    }

    pub fn fail_request(&mut self, error: String) {
        self.loading_game = false;
        self.submitting_guess = false;
        self.error = Some(error);
    }

    pub fn start_game(&mut self, response: NewGameResponse) {
        self.game_id = Some(response.game_id);
        self.metadata_difficulty = self.settings.difficulty;
        self.metadata = response.metadata;
        self.playable_verse_count = response.playable_verse_count;
        self.total_verse_count = response.total_verse_count;
        self.last_game_new_best = false;
        self.rounds = response
            .rounds
            .into_iter()
            .map(|prompt| Round {
                text: prompt.text,
                guess: None,
            })
            .collect();
        self.current_round_index = 0;
        self.reset_guess_path();
        self.finished = false;
        self.loading_game = false;
        self.submitting_guess = false;
        self.error = None;
        self.screen = Screen::Playing;
    }

    pub fn change_settings(&mut self) {
        self.screen = Screen::Setup;
        self.finished = false;
        self.last_game_new_best = false;
        self.rounds.clear();
        self.current_round_index = 0;
        self.game_id = None;
        self.clear_guess();
    }

    pub fn set_round_count(&mut self, round_count: usize) {
        self.settings.round_count = round_count;
    }

    pub fn set_difficulty(&mut self, difficulty: Difficulty) {
        if self.settings.difficulty != difficulty {
            self.settings.difficulty = difficulty;
            self.clear_guess();
        }
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
        if !self.metadata_ready() {
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
            let books = self.books_for(canon);
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

    pub fn guess_request(&self) -> Option<GuessRequest> {
        Some(GuessRequest {
            round_index: self.current_round_index,
            canon: self.selected_canon?,
            book: self.selected_book.clone()?,
            chapter: self.selected_chapter?,
        })
    }

    pub fn begin_submitting_guess(&mut self) {
        self.submitting_guess = true;
        self.error = None;
    }

    pub fn apply_guess(&mut self, response: GuessResponse) {
        if let Some(round) = self.rounds.get_mut(self.current_round_index) {
            round.guess = Some(GuessResult {
                answer: response.answer,
                guess: response.guess,
                score: response.score,
                chapter_verses: response.chapter_verses,
            });
        }
        self.submitting_guess = false;
    }

    pub fn current_result_marked_for_review(&self) -> bool {
        self.current_round()
            .guess
            .as_ref()
            .map(|guess| self.stats.is_marked_for_review(&guess.answer))
            .unwrap_or(false)
    }

    pub fn toggle_current_result_review(&mut self) -> bool {
        let Some(round) = self.rounds.get(self.current_round_index) else {
            return false;
        };
        let Some(guess) = round.guess.as_ref() else {
            return false;
        };

        self.stats.toggle_review_item(ReviewItem {
            reference: guess.answer.clone(),
            text: round.text.clone(),
            score: guess.score.points,
        })
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

    pub fn books_for(&self, canon: Canon) -> Vec<BookInfo> {
        self.metadata
            .iter()
            .find(|item| item.canon == canon)
            .map(|item| item.books.clone())
            .unwrap_or_default()
    }

    pub fn chapters_for(&self, canon: Canon, book: &str) -> Vec<u16> {
        self.metadata
            .iter()
            .find(|item| item.canon == canon)
            .and_then(|item| item.books.iter().find(|candidate| candidate.name == book))
            .map(|book| book.chapters.clone())
            .unwrap_or_default()
    }

    pub fn verse_count_for_difficulty(&self) -> usize {
        if self.metadata_ready() {
            self.playable_verse_count
        } else {
            0
        }
    }

    pub fn total_verse_count(&self) -> usize {
        if self.metadata_ready() {
            self.total_verse_count
        } else {
            0
        }
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
                        answer_book: guess.answer.book.clone(),
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
    pub text: String,
    pub guess: Option<GuessResult>,
}

#[derive(Clone, PartialEq)]
pub struct GuessResult {
    pub answer: Reference,
    pub guess: GuessReference,
    pub score: Score,
    pub chapter_verses: Vec<ChapterVerse>,
}
