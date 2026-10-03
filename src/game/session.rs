use crate::api::{AdvanceGameResponse, GameSnapshotResponse, GuessResponse, SnapshotRound};
use crate::scriptures::{Difficulty, GameScope};

/// A game loaded from the server, from its first round through the final score.
#[derive(Clone, Debug, PartialEq)]
pub struct PlaySession {
    pub id: String,
    /// Shown instead of the scope label, such as a study set name.
    pub label: Option<String>,
    pub difficulty: Difficulty,
    pub scope: GameScope,
    pub max_total_score: u32,
    pub rounds: Vec<SnapshotRound>,
    pub current_round_index: usize,
    pub finished: bool,
}

impl PlaySession {
    pub fn new(snapshot: &GameSnapshotResponse, label: Option<String>) -> Self {
        Self {
            id: snapshot.game_id.clone(),
            label,
            difficulty: snapshot.summary.difficulty,
            scope: snapshot.summary.scope.clone(),
            max_total_score: snapshot.max_total_score,
            current_round_index: snapshot
                .current_round_index
                .min(snapshot.rounds.len().saturating_sub(1)),
            rounds: snapshot.rounds.clone(),
            finished: snapshot.finished,
        }
    }

    pub fn current_round(&self) -> &SnapshotRound {
        &self.rounds[self.current_round_index]
    }

    pub fn is_last_round(&self) -> bool {
        self.current_round_index + 1 == self.rounds.len()
    }

    pub fn total_score(&self) -> u32 {
        self.rounds
            .iter()
            .filter_map(|round| round.guess.as_ref().map(|guess| guess.score.points))
            .sum()
    }

    pub fn apply_guess(&mut self, guess: GuessResponse) {
        if let Some(round) = self.rounds.get_mut(self.current_round_index) {
            round.guess = Some(guess);
        }
    }

    /// Returns whether a new round started.
    pub fn apply_advance(&mut self, response: AdvanceGameResponse) -> bool {
        if response.finished {
            self.finished = true;
            return false;
        }
        self.current_round_index = response
            .current_round_index
            .min(self.rounds.len().saturating_sub(1));
        true
    }
}
