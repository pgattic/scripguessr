#[cfg(any(test, target_arch = "wasm32"))]
mod built_in;

use serde::{Deserialize, Serialize};

#[cfg(not(target_arch = "wasm32"))]
use crate::scriptures::Reference;
use crate::scriptures::{BookScope, Canon, CanonScope, ChapterRef, GameMode, GameScope};
#[cfg(any(test, target_arch = "wasm32"))]
pub use built_in::built_in_study_sets;

pub const MAX_VERSES_PER_PASSAGE: usize = 200;

#[derive(Clone, Debug, Deserialize, Hash, PartialEq, Eq, Serialize)]
pub struct StudyPassage {
    pub canon: Canon,
    pub book: String,
    pub chapter: u16,
    pub verses: Vec<u16>,
}

impl StudyPassage {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn single(reference: &Reference) -> Self {
        Self {
            canon: reference.canon,
            book: reference.book.clone(),
            chapter: reference.chapter,
            verses: vec![reference.verse],
        }
    }

    #[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
    pub fn label(&self) -> String {
        let verses = compact_verses(&self.verses);
        format!("{} {}:{}", self.book, self.chapter, verses)
    }

    pub fn chapter_ref(&self) -> ChapterRef {
        ChapterRef {
            canon: self.canon,
            book: self.book.clone(),
            chapter: self.chapter,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct StudySet {
    pub id: String,
    pub name: String,
    pub passages: Vec<StudyPassage>,
    pub guess_scope: StudyGuessScope,
    pub prompt_policy: PromptPolicy,
}

impl StudySet {
    pub fn resolved_guess_scope(&self) -> GameScope {
        match &self.guess_scope {
            StudyGuessScope::BooksInSet => scope_for_passages(&self.passages),
            StudyGuessScope::FullCanons => full_canons_for_passages(&self.passages),
            StudyGuessScope::AllStandardWorks => GameMode::AllStandardWorks.scope(),
            StudyGuessScope::Custom(scope) => scope.clone(),
        }
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn guess_scope_covers_passages(&self) -> bool {
        let scope = self.resolved_guess_scope();
        self.passages
            .iter()
            .all(|passage| scope.includes_book(passage.canon, &passage.book))
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub enum PromptPolicy {
    #[default]
    Automatic,
    SingleVerse,
    WholePassage,
}

impl PromptPolicy {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub const ALL: [Self; 3] = [Self::Automatic, Self::SingleVerse, Self::WholePassage];

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn label(self) -> &'static str {
        match self {
            Self::Automatic => "Automatic",
            Self::SingleVerse => "One verse",
            Self::WholePassage => "Whole passage",
        }
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn description(self) -> &'static str {
        match self {
            Self::Automatic => "Short passages stay together; longer passages use one verse.",
            Self::SingleVerse => "Each round uses one randomly selected verse from its passage.",
            Self::WholePassage => "Each round shows every verse in its passage.",
        }
    }

    pub fn shows_whole_passage(self, passage: &StudyPassage) -> bool {
        match self {
            Self::Automatic => passage.verses.len() <= 3,
            Self::SingleVerse => false,
            Self::WholePassage => true,
        }
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn behavior_label(self, passage: &StudyPassage) -> &'static str {
        if self.shows_whole_passage(passage) {
            "Whole passage"
        } else {
            "Random verse"
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub enum StudyGuessScope {
    BooksInSet,
    #[default]
    FullCanons,
    AllStandardWorks,
    Custom(GameScope),
}

impl StudyGuessScope {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn label(&self) -> &'static str {
        match self {
            Self::BooksInSet => "Books in set",
            Self::FullCanons => "Full canons",
            Self::AllStandardWorks => "All standard works",
            Self::Custom(_) => "Custom",
        }
    }
}

fn scope_for_passages(passages: &[StudyPassage]) -> GameScope {
    let canons = Canon::ALL
        .iter()
        .copied()
        .filter_map(|canon| {
            let books = passages
                .iter()
                .filter(|passage| passage.canon == canon)
                .map(|passage| passage.book.clone())
                .fold(Vec::new(), |mut books, book| {
                    if !books.contains(&book) {
                        books.push(book);
                    }
                    books
                });
            (!books.is_empty()).then_some(CanonScope {
                canon,
                books: BookScope::Selected(books),
            })
        })
        .collect();
    GameScope { canons }
}

fn full_canons_for_passages(passages: &[StudyPassage]) -> GameScope {
    GameScope {
        canons: Canon::ALL
            .iter()
            .copied()
            .filter(|canon| passages.iter().any(|passage| passage.canon == *canon))
            .map(|canon| CanonScope {
                canon,
                books: BookScope::All,
            })
            .collect(),
    }
}

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
fn compact_verses(verses: &[u16]) -> String {
    let mut verses = verses.to_vec();
    verses.sort_unstable();
    verses.dedup();
    let mut parts = Vec::new();
    let mut index = 0;
    while index < verses.len() {
        let start = verses[index];
        let mut end = start;
        while index + 1 < verses.len() && verses[index + 1] == end + 1 {
            index += 1;
            end = verses[index];
        }
        if start == end {
            parts.push(start.to_string());
        } else {
            parts.push(format!("{start}-{end}"));
        }
        index += 1;
    }
    parts.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_discontinuous_verse_ranges() {
        let passage = StudyPassage {
            canon: Canon::DoctrineAndCovenants,
            book: "D&C".to_string(),
            chapter: 121,
            verses: vec![36, 41, 42],
        };
        assert_eq!(passage.label(), "D&C 121:36, 41-42");
    }

    #[test]
    fn books_in_set_remains_available_for_focused_custom_sets() {
        let set = StudySet {
            id: "alma".to_string(),
            name: "Alma".to_string(),
            passages: vec![StudyPassage {
                canon: Canon::BookOfMormon,
                book: "Alma".to_string(),
                chapter: 32,
                verses: vec![21],
            }],
            guess_scope: StudyGuessScope::BooksInSet,
            prompt_policy: PromptPolicy::Automatic,
        };
        let scope = set.resolved_guess_scope();

        assert!(scope.includes_book(Canon::BookOfMormon, "Alma"));
        assert!(!scope.includes_book(Canon::BookOfMormon, "Jacob"));
    }
}
