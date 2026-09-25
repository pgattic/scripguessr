use serde::{Deserialize, Serialize};

use crate::scoring::Score;
use crate::scriptures::{BookInfo, Canon, Difficulty, GameScope};
use crate::study_sets::{PromptPolicy, StudyPassage};

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

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NewGameResponse {
    pub game_id: String,
    pub rounds: Vec<RoundPrompt>,
    pub difficulty: Difficulty,
    pub scope: GameScope,
    pub metadata: Vec<CanonMetadata>,
    pub playable_verse_count: usize,
    pub total_verse_count: usize,
    pub max_total_score: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct GameSnapshotResponse {
    pub game_id: String,
    pub rounds: Vec<SnapshotRound>,
    pub current_round_index: usize,
    pub finished: bool,
    pub difficulty: Difficulty,
    pub scope: GameScope,
    pub metadata: Vec<CanonMetadata>,
    pub playable_verse_count: usize,
    pub total_verse_count: usize,
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

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MetadataResponse {
    pub difficulty: Difficulty,
    pub scope: GameScope,
    pub metadata: Vec<CanonMetadata>,
    pub playable_verse_count: usize,
    pub total_verse_count: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RoundPrompt {
    pub text: String,
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
    pub canon: Canon,
    pub book: String,
    pub chapter: u16,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct GuessResponse {
    pub answer: StudyPassage,
    pub source_passage: StudyPassage,
    pub guess: GuessReference,
    pub score: Score,
    pub chapter_verses: Vec<ChapterVerse>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct GuessReference {
    pub canon: Canon,
    pub book: String,
    pub chapter: u16,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ChapterVerse {
    pub verse: u16,
    pub text: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ChapterRequest {
    pub canon: Canon,
    pub book: String,
    pub chapter: u16,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ChapterResponse {
    pub verses: Vec<ChapterVerse>,
}
