use crate::api::{
    AdvanceGameResponse, CanonMetadata, ChapterVerse, GameSnapshotResponse, GuessReference,
    GuessRequest, GuessResponse, MetadataRequest, MetadataResponse, NewGameRequest,
    NewGameResponse,
};
use crate::scoring::{MAX_SCORE, Score};
use crate::scriptures::{BookInfo, BookScope, Canon, CanonScope, Difficulty, GameMode, GameScope};
use crate::stats::{FinishedGame, FinishedRound, ReviewItem, Stats};
use crate::study_sets::{CustomStudySets, PromptPolicy, StudyGuessScope, StudyPassage, StudySet};

#[derive(Clone, PartialEq)]
pub struct Game {
    pub game_id: Option<String>,
    pub metadata: Vec<CanonMetadata>,
    pub study_metadata: Vec<CanonMetadata>,
    pub metadata_difficulty: Difficulty,
    pub metadata_scope: GameScope,
    pub playable_verse_count: usize,
    pub total_verse_count: usize,
    pub settings: GameSettings,
    pub stats: Stats,
    pub custom_study_sets: CustomStudySets,
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
    pub active_game: Option<ActiveGame>,
}

impl Game {
    pub fn new() -> Self {
        let settings = GameSettings::default();
        Self {
            game_id: None,
            metadata: Vec::new(),
            study_metadata: Vec::new(),
            metadata_difficulty: Difficulty::Normal,
            metadata_scope: settings.scope.clone(),
            playable_verse_count: 0,
            total_verse_count: 0,
            settings,
            stats: Stats::load(),
            custom_study_sets: CustomStudySets::load(),
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
            active_game: None,
        }
    }

    pub fn metadata_request(&self) -> MetadataRequest {
        MetadataRequest {
            difficulty: self.settings.difficulty,
            scope: self.settings.scope.clone(),
        }
    }

    pub fn study_metadata_request(&self) -> MetadataRequest {
        MetadataRequest {
            difficulty: Difficulty::Normal,
            scope: GameMode::AllStandardWorks.scope(),
        }
    }

    pub fn apply_study_metadata(&mut self, response: MetadataResponse) {
        self.study_metadata = response.metadata;
    }

    pub fn new_game_request(&self) -> NewGameRequest {
        NewGameRequest::Random {
            round_count: self.settings.round_count,
            difficulty: self.settings.difficulty,
            scope: self.settings.scope.clone(),
        }
    }

    pub fn review_game_request(&self) -> Option<NewGameRequest> {
        let passages = self
            .stats
            .review_items
            .iter()
            .map(ReviewItem::passage)
            .collect::<Vec<_>>();
        let round_count = passages.len();
        self.study_set_game_request(
            &StudySet {
                id: "review".to_string(),
                name: "Marked verses".to_string(),
                passages,
                guess_scope: StudyGuessScope::FullCanons,
                prompt_policy: PromptPolicy::WholePassage,
            },
            round_count,
        )
    }

    pub fn study_set_game_request(
        &self,
        set: &StudySet,
        round_count: usize,
    ) -> Option<NewGameRequest> {
        if set.passages.is_empty() {
            return None;
        }
        Some(NewGameRequest::Study {
            round_count: round_count.min(set.passages.len()),
            difficulty: self.settings.difficulty,
            scope: set.resolved_guess_scope(),
            passages: set.passages.clone(),
            prompt_policy: set.prompt_policy,
        })
    }

    pub fn apply_metadata(&mut self, response: MetadataResponse) {
        if self.screen != Screen::Setup {
            return;
        }
        if response.difficulty != self.settings.difficulty || response.scope != self.settings.scope
        {
            return;
        }

        self.metadata_difficulty = response.difficulty;
        self.metadata_scope = response.scope;
        self.metadata = response.metadata;
        self.playable_verse_count = response.playable_verse_count;
        self.total_verse_count = response.total_verse_count;
        self.advance_past_single_option_steps();
    }

    pub fn metadata_ready(&self) -> bool {
        self.metadata_current() && self.playable_verse_count > 0
    }

    pub fn metadata_current(&self) -> bool {
        let (difficulty, scope) = self
            .active_game
            .as_ref()
            .map(|game| (game.difficulty, &game.scope))
            .unwrap_or((self.settings.difficulty, &self.settings.scope));
        self.metadata_difficulty == difficulty
            && self.metadata_scope == *scope
            && (!self.metadata.is_empty() || scope.canons.is_empty())
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
        self.start_game_named(response, None);
    }

    pub fn start_game_named(&mut self, response: NewGameResponse, label: Option<String>) {
        self.active_game = Some(ActiveGame {
            round_count: response.rounds.len(),
            difficulty: response.difficulty,
            scope: response.scope.clone(),
            max_total_score: response.max_total_score,
            label: label.clone(),
        });
        self.game_id = Some(response.game_id);
        self.metadata_difficulty = response.difficulty;
        self.metadata_scope = response.scope;
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

    pub fn restore_game(&mut self, response: GameSnapshotResponse) {
        self.active_game = Some(ActiveGame {
            round_count: response.rounds.len(),
            difficulty: response.difficulty,
            scope: response.scope.clone(),
            max_total_score: response.max_total_score,
            label: None,
        });
        self.game_id = Some(response.game_id);
        self.metadata_difficulty = response.difficulty;
        self.metadata_scope = response.scope;
        self.metadata = response.metadata;
        self.playable_verse_count = response.playable_verse_count;
        self.total_verse_count = response.total_verse_count;
        self.rounds = response
            .rounds
            .into_iter()
            .map(|round| Round {
                text: round.text,
                guess: round.guess.map(GuessResult::from),
            })
            .collect();
        self.current_round_index = response
            .current_round_index
            .min(self.rounds.len().saturating_sub(1));
        self.finished = response.finished;
        self.loading_game = false;
        self.submitting_guess = false;
        self.error = None;
        self.screen = Screen::Playing;
        self.reset_guess_path();
    }

    pub fn change_settings(&mut self) {
        self.screen = Screen::Setup;
        self.finished = false;
        self.last_game_new_best = false;
        self.rounds.clear();
        self.current_round_index = 0;
        self.game_id = None;
        self.active_game = None;
        self.clear_guess();
    }

    pub fn create_study_set(&mut self) -> String {
        self.save_custom_study_set(StudySet {
            id: String::new(),
            name: "Untitled set".to_string(),
            passages: Vec::new(),
            guess_scope: StudyGuessScope::FullCanons,
            prompt_policy: PromptPolicy::Automatic,
        })
    }

    pub fn save_custom_study_set(&mut self, mut set: StudySet) -> String {
        let id = loop {
            let candidate = format!("custom-{:016x}", rand::random::<u64>());
            if self
                .custom_study_sets
                .sets
                .iter()
                .all(|set| set.id != candidate)
            {
                break candidate;
            }
        };
        set.id = id.clone();
        self.custom_study_sets.sets.push(set);
        self.custom_study_sets.save();
        id
    }

    pub fn rename_study_set(&mut self, id: &str, name: String) {
        if let Some(set) = self
            .custom_study_sets
            .sets
            .iter_mut()
            .find(|set| set.id == id)
        {
            set.name = name;
            self.custom_study_sets.save();
        }
    }

    pub fn add_study_passage(&mut self, id: &str, passage: StudyPassage) {
        if let Some(set) = self
            .custom_study_sets
            .sets
            .iter_mut()
            .find(|set| set.id == id)
            && !set.passages.contains(&passage)
        {
            set.passages.push(passage);
            self.custom_study_sets.save();
        }
    }

    pub fn set_study_guess_scope(&mut self, id: &str, guess_scope: StudyGuessScope) {
        if let Some(set) = self
            .custom_study_sets
            .sets
            .iter_mut()
            .find(|set| set.id == id)
        {
            set.guess_scope = guess_scope;
            self.custom_study_sets.save();
        }
    }

    pub fn set_study_prompt_policy(&mut self, id: &str, prompt_policy: PromptPolicy) {
        if let Some(set) = self
            .custom_study_sets
            .sets
            .iter_mut()
            .find(|set| set.id == id)
        {
            set.prompt_policy = prompt_policy;
            self.custom_study_sets.save();
        }
    }

    pub fn remove_study_passage(&mut self, id: &str, index: usize) {
        if let Some(set) = self
            .custom_study_sets
            .sets
            .iter_mut()
            .find(|set| set.id == id)
            && index < set.passages.len()
        {
            set.passages.remove(index);
            self.custom_study_sets.save();
        }
    }

    pub fn delete_study_set(&mut self, id: &str) {
        self.custom_study_sets.sets.retain(|set| set.id != id);
        self.custom_study_sets.save();
    }

    pub fn set_round_count(&mut self, round_count: usize) {
        self.settings.round_count = round_count;
    }

    pub fn set_difficulty(&mut self, difficulty: Difficulty) {
        if self.settings.difficulty != difficulty {
            self.settings.difficulty = difficulty;
            self.clear_guess();
            self.apply_empty_scope_metadata();
        }
    }

    pub fn set_preset(&mut self, mode: GameMode) {
        let scope = mode.scope();
        if self.settings.scope != scope {
            self.settings.scope = scope;
            self.clear_guess();
        }
    }

    pub fn toggle_canon(&mut self, canon: Canon) {
        if let Some(index) = self
            .settings
            .scope
            .canons
            .iter()
            .position(|scope| scope.canon == canon)
        {
            self.settings.scope.canons.remove(index);
        } else {
            self.settings.scope.canons.push(CanonScope {
                canon,
                books: BookScope::All,
            });
            self.settings
                .scope
                .canons
                .sort_by_key(|scope| Canon::ALL.iter().position(|item| item == &scope.canon));
        }

        self.clear_guess();
        self.apply_empty_scope_metadata();
    }

    pub fn set_canon_book_scope(&mut self, canon: Canon, books: BookScope) {
        if let Some(canon_scope) = self
            .settings
            .scope
            .canons
            .iter_mut()
            .find(|scope| scope.canon == canon)
        {
            canon_scope.books = books;
            self.clear_guess();
            self.apply_empty_scope_metadata();
        }
    }

    fn apply_empty_scope_metadata(&mut self) {
        if !self.settings.scope.canons.is_empty() {
            return;
        }

        self.metadata.clear();
        self.metadata_difficulty = self.settings.difficulty;
        self.metadata_scope = self.settings.scope.clone();
        self.playable_verse_count = 0;
        self.total_verse_count = 0;
        self.error = None;
    }

    pub fn current_round(&self) -> &Round {
        &self.rounds[self.current_round_index]
    }

    pub fn is_last_round(&self) -> bool {
        self.current_round_index + 1 == self.rounds.len()
    }

    pub fn max_total_score(&self) -> u32 {
        self.active_game
            .as_ref()
            .map(|game| game.max_total_score)
            .unwrap_or(self.settings.round_count as u32 * MAX_SCORE)
    }

    pub fn guess_scope(&self) -> &GameScope {
        self.active_game
            .as_ref()
            .map(|game| &game.scope)
            .unwrap_or(&self.settings.scope)
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

        let scope = self.guess_scope();
        if self.active_step == GuessStep::Canon && scope.canons.len() == 1 {
            self.selected_canon = scope.canons.first().map(|scope| scope.canon);
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
                source_passage: response.source_passage,
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
            passage: guess.answer.clone(),
            text: round.text.clone(),
            score: guess.score.points,
        })
    }

    pub fn remove_review_item(&mut self, passage: &StudyPassage) -> bool {
        self.stats.remove_review_item(passage)
    }

    pub fn apply_advance(&mut self, response: AdvanceGameResponse) {
        if response.finished && !self.finished {
            self.finished = true;
            self.record_finished_game();
        } else if !response.finished {
            self.current_round_index = response
                .current_round_index
                .min(self.rounds.len().saturating_sub(1));
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
        let book_scope = self
            .guess_scope()
            .canon_scope(canon)
            .map(|scope| &scope.books)
            .unwrap_or(&BookScope::All);

        self.metadata
            .iter()
            .find(|item| item.canon == canon)
            .map(|item| match book_scope {
                BookScope::All => item.books.clone(),
                BookScope::Selected(selected) => item
                    .books
                    .iter()
                    .filter(|book| selected.iter().any(|name| name == &book.name))
                    .cloned()
                    .collect(),
            })
            .unwrap_or_default()
    }

    pub fn all_books_for(&self, canon: Canon) -> Vec<BookInfo> {
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
        if self.metadata_current() {
            self.playable_verse_count
        } else {
            0
        }
    }

    pub fn total_verse_count(&self) -> usize {
        if self.metadata_current() {
            self.total_verse_count
        } else {
            0
        }
    }

    fn record_finished_game(&mut self) {
        let finished_game = FinishedGame {
            difficulty: self
                .active_game
                .as_ref()
                .map(|game| game.difficulty)
                .unwrap_or(self.settings.difficulty),
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
pub struct ActiveGame {
    pub round_count: usize,
    pub difficulty: Difficulty,
    pub scope: GameScope,
    pub max_total_score: u32,
    pub label: Option<String>,
}

#[derive(Clone, PartialEq)]
pub struct GameSettings {
    pub round_count: usize,
    pub difficulty: Difficulty,
    pub scope: GameScope,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            round_count: 5,
            difficulty: Difficulty::Normal,
            scope: GameScope::default(),
        }
    }
}

impl GameSettings {
    pub fn selection_label(&self) -> String {
        if self.scope.canons.is_empty() {
            return "No scope selected".to_string();
        }

        if let Some(mode) = GameMode::ALL
            .iter()
            .copied()
            .find(|mode| mode.scope() == self.scope)
        {
            return mode.label().to_string();
        }

        let parts = self
            .scope
            .canons
            .iter()
            .map(scope_label)
            .collect::<Vec<_>>();

        if self.scope.canons.len() == 1 {
            return parts
                .into_iter()
                .next()
                .unwrap_or_else(|| "Custom scope".to_string());
        }

        if parts.len() <= 3 && parts.iter().all(|part| part.len() <= 24) {
            return parts.join(" + ");
        }

        let selected_book_count = self
            .scope
            .canons
            .iter()
            .filter_map(|scope| match &scope.books {
                BookScope::All => None,
                BookScope::Selected(books) => Some(books.len()),
            })
            .sum::<usize>();
        let all_canon_count = self
            .scope
            .canons
            .iter()
            .filter(|scope| scope.books == BookScope::All)
            .count();

        if selected_book_count > 0 && all_canon_count > 0 {
            format!("Custom: {selected_book_count} books + {all_canon_count} full canons")
        } else if selected_book_count > 0 {
            format!(
                "Custom: {selected_book_count} books across {} canons",
                self.scope.canons.len()
            )
        } else {
            format!("Custom: {} canons", self.scope.canons.len())
        }
    }
}

fn scope_label(scope: &CanonScope) -> String {
    match &scope.books {
        BookScope::All => scope.canon.label().to_string(),
        BookScope::Selected(books) if books.is_empty() => format!("No {}", scope.canon.label()),
        BookScope::Selected(books) if books.len() == 1 => format!("{} only", books[0]),
        BookScope::Selected(books) if books.len() <= 3 => books.join(" + "),
        BookScope::Selected(books) => format!("{} {} books", books.len(), scope.canon.label()),
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
    pub answer: StudyPassage,
    pub source_passage: StudyPassage,
    pub guess: GuessReference,
    pub score: Score,
    pub chapter_verses: Vec<ChapterVerse>,
}

impl From<GuessResponse> for GuessResult {
    fn from(response: GuessResponse) -> Self {
        Self {
            answer: response.answer,
            source_passage: response.source_passage,
            guess: response.guess,
            score: response.score,
            chapter_verses: response.chapter_verses,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::RoundPrompt;

    #[test]
    fn starting_a_study_game_preserves_setup_preferences() {
        let mut game = Game::new();
        let settings = game.settings.clone();
        let study_scope = GameMode::AllStandardWorks.scope();

        game.start_game_named(
            NewGameResponse {
                game_id: "study-game".to_string(),
                rounds: vec![RoundPrompt {
                    text: "A verse".to_string(),
                }],
                difficulty: Difficulty::Hard,
                scope: study_scope.clone(),
                metadata: Vec::new(),
                playable_verse_count: 1,
                total_verse_count: 1,
                max_total_score: MAX_SCORE,
            },
            Some("Study set".to_string()),
        );

        assert_eq!(game.settings, settings);
        assert_eq!(game.guess_scope(), &study_scope);
        assert_eq!(game.active_game.as_ref().unwrap().round_count, 1);

        game.change_settings();
        assert_eq!(game.settings, settings);
        assert!(game.active_game.is_none());
    }
}
