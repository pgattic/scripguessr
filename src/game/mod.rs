#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

mod account;
pub mod catalog;
mod guess_path;
mod session;
mod settings;

pub use account::{Account, ReviewToggle};
pub use guess_path::{GuessPath, GuessStep};
pub use session::PlaySession;
pub use settings::GameSettings;

use crate::api::{
    AdvanceGameResponse, CanonMetadata, GameSnapshotResponse, GuessRequest, GuessResponse,
    MetadataRequest, NewGameRequest, ScopeSummary, SnapshotRound,
};
use crate::scoring::max_total_score;
use crate::scriptures::{BookInfo, Canon, Difficulty, GameMode, GameScope};
use crate::stats::ReviewItem;
use crate::study_sets::{PromptPolicy, StudyGuessScope, StudySet};

/// Client state shared by every page through a Dioxus context signal.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Game {
    pub settings: GameSettings,
    /// Verse pool for the active game, or for `settings` while on setup.
    pub catalog: Option<ScopeSummary>,
    /// Every book in the standard works, for study sets and the atlas.
    pub study_catalog: Vec<CanonMetadata>,
    pub session: Option<PlaySession>,
    pub guess: GuessPath,
    pub account: Account,
    pub loading_game: bool,
    pub submitting_guess: bool,
    pub error: Option<String>,
}

impl Game {
    pub fn metadata_request(&self) -> MetadataRequest {
        MetadataRequest {
            difficulty: self.settings.difficulty,
            scope: self.settings.scope.clone(),
        }
    }

    pub fn study_catalog_request() -> MetadataRequest {
        MetadataRequest {
            difficulty: Difficulty::Easy,
            scope: GameMode::AllStandardWorks.scope(),
        }
    }

    pub fn new_game_request(&self) -> NewGameRequest {
        NewGameRequest::Random {
            round_count: self.settings.round_count,
            difficulty: self.settings.difficulty,
            scope: self.settings.scope.clone(),
        }
    }

    pub fn study_game_request(&self, set: &StudySet, round_count: usize) -> Option<NewGameRequest> {
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

    pub fn review_study_set(&self) -> StudySet {
        StudySet {
            id: "review".to_string(),
            name: "Marked verses".to_string(),
            passages: self
                .account
                .review_items
                .iter()
                .map(|item| item.passage.clone())
                .collect(),
            guess_scope: StudyGuessScope::FullCanons,
            prompt_policy: PromptPolicy::WholePassage,
        }
    }

    pub fn apply_metadata(&mut self, summary: ScopeSummary) {
        let current_request = self.metadata_request();
        if self.session.is_some()
            || summary.difficulty != current_request.difficulty
            || summary.scope != current_request.scope
        {
            return;
        }
        self.catalog = Some(summary);
        self.skip_single_guess_choices();
    }

    pub fn apply_study_catalog(&mut self, summary: ScopeSummary) {
        self.study_catalog = summary.metadata;
    }

    pub fn metadata_current(&self) -> bool {
        let scope = self.guess_scope();
        self.catalog.as_ref().is_some_and(|catalog| {
            catalog.difficulty == self.difficulty()
                && catalog.scope == *scope
                && (!catalog.metadata.is_empty() || scope.canons.is_empty())
        })
    }

    pub fn metadata_ready(&self) -> bool {
        self.metadata_current() && self.playable_verse_count() > 0
    }

    pub fn playable_verse_count(&self) -> usize {
        self.current_catalog()
            .map_or(0, |catalog| catalog.playable_verse_count)
    }

    pub fn total_verse_count(&self) -> usize {
        self.current_catalog()
            .map_or(0, |catalog| catalog.total_verse_count)
    }

    pub fn begin_starting_game(&mut self) {
        self.loading_game = true;
        self.error = None;
    }

    pub fn begin_submitting_guess(&mut self) {
        self.submitting_guess = true;
        self.error = None;
    }

    pub fn fail_request(&mut self, error: String) {
        self.loading_game = false;
        self.submitting_guess = false;
        self.error = Some(error);
    }

    pub fn load_session(&mut self, snapshot: GameSnapshotResponse, label: Option<String>) {
        self.session = Some(PlaySession::new(&snapshot, label));
        self.catalog = Some(snapshot.summary);
        self.loading_game = false;
        self.submitting_guess = false;
        self.error = None;
        self.reset_guess();
    }

    pub fn change_settings(&mut self) {
        self.session = None;
        self.reset_guess();
    }

    pub fn set_round_count(&mut self, round_count: usize) {
        self.settings.round_count = round_count;
    }

    pub fn set_difficulty(&mut self, difficulty: Difficulty) {
        if self.settings.difficulty != difficulty {
            self.settings.difficulty = difficulty;
            self.settings_changed();
        }
    }

    pub fn set_scope(&mut self, scope: GameScope) {
        if self.settings.scope != scope {
            self.settings.scope = scope;
            self.settings_changed();
        }
    }

    pub fn session(&self) -> Option<&PlaySession> {
        self.session.as_ref()
    }

    pub fn current_round(&self) -> Option<&SnapshotRound> {
        self.session().map(PlaySession::current_round)
    }

    pub fn total_score(&self) -> u32 {
        self.session().map_or(0, PlaySession::total_score)
    }

    pub fn max_total_score(&self) -> u32 {
        self.session().map_or_else(
            || max_total_score(self.settings.round_count),
            |session| session.max_total_score,
        )
    }

    pub fn label(&self) -> String {
        self.session()
            .and_then(|session| session.label.clone())
            .unwrap_or_else(|| self.settings.selection_label())
    }

    pub fn guess_scope(&self) -> &GameScope {
        self.session()
            .map_or(&self.settings.scope, |session| &session.scope)
    }

    pub fn books_for(&self, canon: Canon) -> Vec<BookInfo> {
        catalog::books_in_scope(self.metadata(), self.guess_scope(), canon)
    }

    pub fn chapters_for(&self, canon: Canon, book: &str) -> Vec<u16> {
        catalog::chapters(self.metadata(), canon, book)
    }

    pub fn select_canon(&mut self, canon: Canon) {
        self.guess.select_canon(canon);
        self.skip_single_guess_choices();
    }

    pub fn select_book(&mut self, book: String) {
        self.guess.select_book(book);
        self.skip_single_guess_choices();
    }

    pub fn reset_guess(&mut self) {
        self.guess = GuessPath::default();
        self.skip_single_guess_choices();
    }

    pub fn guess_request(&self) -> Option<GuessRequest> {
        Some(GuessRequest {
            round_index: self.session()?.current_round_index,
            chapter: self.guess.chapter_ref()?,
        })
    }

    pub fn apply_guess(&mut self, guess: GuessResponse) {
        if let Some(session) = self.session.as_mut() {
            session.apply_guess(guess);
        }
        self.submitting_guess = false;
    }

    pub fn apply_advance(&mut self, response: AdvanceGameResponse) {
        let next_round = self
            .session
            .as_mut()
            .is_some_and(|session| session.apply_advance(response));
        if next_round {
            self.reset_guess();
        }
    }

    pub fn current_result_marked_for_review(&self) -> bool {
        self.current_round()
            .and_then(|round| round.guess.as_ref())
            .is_some_and(|guess| self.account.is_marked(&guess.answer))
    }

    pub fn current_review_item(&self) -> Option<ReviewItem> {
        let round = self.current_round()?;
        let guess = round.guess.as_ref()?;
        Some(ReviewItem {
            passage: guess.answer.clone(),
            text: round.text.clone(),
            score: guess.score.points,
        })
    }

    fn difficulty(&self) -> Difficulty {
        self.session()
            .map_or(self.settings.difficulty, |session| session.difficulty)
    }

    fn metadata(&self) -> &[CanonMetadata] {
        self.catalog
            .as_ref()
            .map(|catalog| catalog.metadata.as_slice())
            .unwrap_or_default()
    }

    fn current_catalog(&self) -> Option<&ScopeSummary> {
        self.metadata_current()
            .then_some(self.catalog.as_ref())
            .flatten()
    }

    fn settings_changed(&mut self) {
        self.reset_guess();
        if self.settings.scope.canons.is_empty() {
            self.catalog = Some(ScopeSummary {
                difficulty: self.settings.difficulty,
                scope: self.settings.scope.clone(),
                metadata: Vec::new(),
                playable_verse_count: 0,
                total_verse_count: 0,
            });
            self.error = None;
        }
    }

    fn skip_single_guess_choices(&mut self) {
        if !self.metadata_ready() {
            return;
        }
        let scope = self
            .session
            .as_ref()
            .map_or(&self.settings.scope, |session| &session.scope);
        let metadata = self
            .catalog
            .as_ref()
            .map(|catalog| catalog.metadata.as_slice())
            .unwrap_or_default();
        self.guess.skip_single_choices(scope, metadata);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(scope: GameScope) -> GameSnapshotResponse {
        GameSnapshotResponse {
            game_id: "study-game".to_string(),
            rounds: vec![SnapshotRound {
                text: "A verse".to_string(),
                guess: None,
            }],
            current_round_index: 0,
            finished: false,
            summary: ScopeSummary {
                difficulty: Difficulty::Hard,
                scope,
                metadata: Vec::new(),
                playable_verse_count: 1,
                total_verse_count: 1,
            },
            max_total_score: max_total_score(1),
        }
    }

    #[test]
    fn starting_a_study_game_preserves_setup_preferences() {
        let mut game = Game::default();
        let settings = game.settings.clone();
        let study_scope = GameMode::AllStandardWorks.scope();

        game.load_session(snapshot(study_scope.clone()), Some("Study set".to_string()));

        assert_eq!(game.settings, settings);
        assert_eq!(game.guess_scope(), &study_scope);
        assert_eq!(game.label(), "Study set");
        assert_eq!(game.session().unwrap().rounds.len(), 1);

        game.change_settings();
        assert_eq!(game.settings, settings);
        assert!(game.session().is_none());
    }

    #[test]
    fn metadata_for_stale_settings_is_ignored() {
        let mut game = Game::default();
        let mut summary = snapshot(GameMode::Bible.scope()).summary;
        summary.difficulty = game.settings.difficulty;

        game.apply_metadata(summary);

        assert!(game.catalog.is_none());
    }

    #[test]
    fn clearing_the_scope_marks_the_empty_pool_current() {
        let mut game = Game::default();

        game.set_scope(GameScope { canons: Vec::new() });

        assert!(game.metadata_current());
        assert!(!game.metadata_ready());
    }
}
