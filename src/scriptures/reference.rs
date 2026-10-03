use std::fmt;

use serde::{Deserialize, Serialize};

use super::Canon;

#[derive(Clone, Debug, Deserialize, Hash, PartialEq, Eq, Serialize)]
pub struct ChapterRef {
    pub canon: Canon,
    pub book: String,
    pub chapter: u16,
}

impl fmt::Display for ChapterRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.book, self.chapter)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, PartialEq)]
pub struct Reference {
    pub canon: Canon,
    pub book: String,
    pub chapter: u16,
    pub verse: u16,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct BookInfo {
    pub name: String,
    pub chapters: Vec<u16>,
    pub chapter_verse_counts: Vec<u16>,
}

impl BookInfo {
    pub fn verse_count(&self, chapter: u16) -> Option<u16> {
        self.chapters
            .iter()
            .position(|candidate| *candidate == chapter)
            .and_then(|index| self.chapter_verse_counts.get(index).copied())
    }
}
