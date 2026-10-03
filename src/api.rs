use serde::{Deserialize, Serialize};

use crate::scoring::Score;
use crate::scriptures::{BookInfo, Canon, ChapterRef, Difficulty, GameMode, GameScope};
use crate::stats::Stats;
use crate::study_sets::{PromptPolicy, StudyPassage, StudySet};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NewGameRequest {
    Random {
        round_count: usize,
        difficulty: Difficulty,
        scope: GameScope,
    },
    Study {
        round_count: usize,
        difficulty: Difficulty,
        scope: GameScope,
        passages: Vec<StudyPassage>,
        prompt_policy: PromptPolicy,
    },
}

/// Returned when a game is created and whenever it is resumed.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct GameSnapshotResponse {
    pub game_id: String,
    pub rounds: Vec<SnapshotRound>,
    pub current_round_index: usize,
    pub finished: bool,
    #[serde(flatten)]
    pub summary: ScopeSummary,
    pub max_total_score: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SnapshotRound {
    pub text: String,
    pub guess: Option<GuessResponse>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AdvanceGameResponse {
    pub current_round_index: usize,
    pub finished: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MetadataRequest {
    pub difficulty: Difficulty,
    pub scope: GameScope,
}

/// The verse pool and book catalog for a difficulty and scope.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ScopeSummary {
    pub difficulty: Difficulty,
    pub scope: GameScope,
    pub metadata: Vec<CanonMetadata>,
    pub playable_verse_count: usize,
    pub total_verse_count: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CanonMetadata {
    pub canon: Canon,
    pub books: Vec<BookInfo>,
    pub playable_verse_count: usize,
    pub total_verse_count: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct GuessRequest {
    pub round_index: usize,
    #[serde(flatten)]
    pub chapter: ChapterRef,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct GuessResponse {
    pub answer: StudyPassage,
    pub source_passage: StudyPassage,
    pub guess: ChapterRef,
    pub score: Score,
    pub chapter_verses: Vec<ChapterVerse>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ChapterVerse {
    pub verse: u16,
    pub text: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ChapterResponse {
    pub verses: Vec<ChapterVerse>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
}

pub const LEADERBOARD_ROUND_COUNTS: [usize; 2] = [5, 10];

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct LeaderboardQuery {
    pub preset: GameMode,
    pub difficulty: Difficulty,
    pub rounds: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct LeaderboardEntry {
    pub rank: u32,
    pub username: String,
    pub score: u32,
    pub possible_score: u32,
    pub completed_at: String,
    pub current_user: bool,
}

impl LeaderboardEntry {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn percent(&self) -> u32 {
        crate::scoring::percent(self.score, self.possible_score).unwrap_or(0)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct LeaderboardResponse {
    pub preset: GameMode,
    pub difficulty: Difficulty,
    pub round_count: usize,
    pub entries: Vec<LeaderboardEntry>,
    pub current_user_entry: Option<LeaderboardEntry>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AccountDataResponse {
    pub stats: Stats,
    pub review_items: Vec<crate::stats::ReviewItem>,
    pub custom_study_sets: Vec<StudySet>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct IdRequest {
    pub id: String,
}
