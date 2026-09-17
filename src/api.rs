use serde::{Deserialize, Serialize};

use crate::scoring::Score;
use crate::scriptures::{BookInfo, Canon, Difficulty, GameScope, Reference};
use crate::study_sets::StudyPassage;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NewGameRequest {
    pub round_count: usize,
    pub difficulty: Difficulty,
    pub scope: GameScope,
    #[serde(default)]
    pub passages: Vec<StudyPassage>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NewGameResponse {
    pub game_id: String,
    pub rounds: Vec<RoundPrompt>,
    pub scope: GameScope,
    pub metadata: Vec<CanonMetadata>,
    pub playable_verse_count: usize,
    pub total_verse_count: usize,
    pub max_total_score: u32,
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
    pub answer: Reference,
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
