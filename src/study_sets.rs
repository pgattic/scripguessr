use serde::{Deserialize, Serialize};

use crate::scriptures::{BookScope, Canon, CanonScope, GameMode, GameScope, Reference};

#[cfg(target_arch = "wasm32")]
const STORAGE_KEY: &str = "scripguessr.study-sets.v1";

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct StudyPassage {
    pub canon: Canon,
    pub book: String,
    pub chapter: u16,
    pub verses: Vec<u16>,
}

impl StudyPassage {
    pub fn single(reference: &Reference) -> Self {
        Self {
            canon: reference.canon,
            book: reference.book.clone(),
            chapter: reference.chapter,
            verses: vec![reference.verse],
        }
    }

    pub fn label(&self) -> String {
        let verses = compact_verses(&self.verses);
        format!("{} {}:{}", self.book, self.chapter, verses)
    }

    pub fn first_reference(&self) -> Option<Reference> {
        Some(Reference {
            canon: self.canon,
            book: self.book.clone(),
            chapter: self.chapter,
            verse: *self.verses.first()?,
        })
    }

    pub fn contains_verse(&self, verse: u16) -> bool {
        self.verses.contains(&verse)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct StudySet {
    pub id: String,
    pub name: String,
    pub passages: Vec<StudyPassage>,
    #[serde(default)]
    pub guess_scope: StudyGuessScope,
    #[serde(default)]
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
    pub const ALL: [Self; 3] = [Self::Automatic, Self::SingleVerse, Self::WholePassage];

    pub fn label(self) -> &'static str {
        match self {
            Self::Automatic => "Automatic",
            Self::SingleVerse => "One verse",
            Self::WholePassage => "Whole passage",
        }
    }

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
    pub fn label(&self) -> &'static str {
        match self {
            Self::BooksInSet => "Books in set",
            Self::FullCanons => "Full canons",
            Self::AllStandardWorks => "All standard works",
            Self::Custom(_) => "Custom",
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct CustomStudySets {
    pub sets: Vec<StudySet>,
}

impl CustomStudySets {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn load() -> Self {
        load_custom_study_sets().unwrap_or_default()
    }

    pub fn save(&self) {
        save_custom_study_sets(self);
    }
}

pub fn scope_for_passages(passages: &[StudyPassage]) -> GameScope {
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

pub fn built_in_study_sets() -> Vec<StudySet> {
    let old_testament = old_testament_mastery();
    let new_testament = new_testament_mastery();
    let book_of_mormon = book_of_mormon_mastery();
    let doctrine_and_covenants = doctrine_and_covenants_mastery();
    let all = [
        &old_testament,
        &new_testament,
        &book_of_mormon,
        &doctrine_and_covenants,
    ]
    .into_iter()
    .flat_map(|set| set.passages.clone())
    .collect();

    vec![
        StudySet {
            id: "doctrinal-mastery-all".to_string(),
            name: "All Doctrinal Mastery".to_string(),
            passages: all,
            guess_scope: StudyGuessScope::AllStandardWorks,
            prompt_policy: PromptPolicy::WholePassage,
        },
        old_testament,
        new_testament,
        book_of_mormon,
        doctrine_and_covenants,
    ]
}

fn passage(canon: Canon, book: &str, chapter: u16, start: u16, end: u16) -> StudyPassage {
    StudyPassage {
        canon,
        book: book.to_string(),
        chapter,
        verses: (start..=end).collect(),
    }
}

fn selected_passage(canon: Canon, book: &str, chapter: u16, verses: &[u16]) -> StudyPassage {
    StudyPassage {
        canon,
        book: book.to_string(),
        chapter,
        verses: verses.to_vec(),
    }
}

fn old_testament_mastery() -> StudySet {
    use Canon::{OldTestament as OT, PearlOfGreatPrice as PGP};
    StudySet {
        id: "doctrinal-mastery-old-testament".to_string(),
        name: "Old Testament Doctrinal Mastery".to_string(),
        passages: vec![
            passage(PGP, "Moses", 1, 39, 39),
            passage(PGP, "Moses", 7, 18, 18),
            passage(PGP, "Abraham", 2, 9, 11),
            passage(PGP, "Abraham", 3, 22, 23),
            passage(OT, "Genesis", 1, 26, 27),
            passage(OT, "Genesis", 2, 24, 24),
            passage(OT, "Genesis", 39, 9, 9),
            passage(OT, "Exodus", 20, 3, 17),
            passage(OT, "Joshua", 24, 15, 15),
            passage(OT, "Psalms", 24, 3, 4),
            passage(OT, "Proverbs", 3, 5, 6),
            passage(OT, "Isaiah", 1, 18, 18),
            passage(OT, "Isaiah", 5, 20, 20),
            passage(OT, "Isaiah", 29, 13, 14),
            passage(OT, "Isaiah", 53, 3, 5),
            passage(OT, "Isaiah", 58, 6, 7),
            passage(OT, "Isaiah", 58, 13, 14),
            passage(OT, "Jeremiah", 1, 4, 5),
            passage(OT, "Ezekiel", 3, 16, 17),
            passage(OT, "Ezekiel", 37, 15, 17),
            passage(OT, "Daniel", 2, 44, 45),
            passage(OT, "Amos", 3, 7, 7),
            passage(OT, "Malachi", 3, 8, 10),
            passage(OT, "Malachi", 4, 5, 6),
        ],
        guess_scope: StudyGuessScope::Custom(GameScope {
            canons: vec![
                CanonScope {
                    canon: OT,
                    books: BookScope::All,
                },
                CanonScope {
                    canon: PGP,
                    books: BookScope::Selected(vec!["Moses".to_string(), "Abraham".to_string()]),
                },
            ],
        }),
        prompt_policy: PromptPolicy::WholePassage,
    }
}

fn new_testament_mastery() -> StudySet {
    use Canon::NewTestament as NT;
    StudySet {
        id: "doctrinal-mastery-new-testament".to_string(),
        name: "New Testament Doctrinal Mastery".to_string(),
        passages: vec![
            passage(NT, "Matthew", 5, 14, 16),
            passage(NT, "Matthew", 11, 28, 30),
            passage(NT, "Matthew", 16, 15, 19),
            passage(NT, "Matthew", 22, 36, 39),
            passage(NT, "Luke", 2, 10, 12),
            passage(NT, "Luke", 22, 19, 20),
            passage(NT, "Luke", 24, 36, 39),
            passage(NT, "John", 3, 5, 5),
            passage(NT, "John", 3, 16, 16),
            passage(NT, "John", 7, 17, 17),
            passage(NT, "John", 17, 3, 3),
            passage(NT, "1 Corinthians", 6, 19, 20),
            passage(NT, "1 Corinthians", 11, 11, 11),
            passage(NT, "1 Corinthians", 15, 20, 22),
            passage(NT, "1 Corinthians", 15, 40, 42),
            passage(NT, "Ephesians", 1, 10, 10),
            passage(NT, "Ephesians", 2, 19, 20),
            passage(NT, "2 Thessalonians", 2, 1, 3),
            passage(NT, "2 Timothy", 3, 15, 17),
            passage(NT, "Hebrews", 12, 9, 9),
            passage(NT, "James", 1, 5, 6),
            passage(NT, "James", 2, 17, 18),
            passage(NT, "1 Peter", 4, 6, 6),
            passage(NT, "Revelation", 20, 12, 12),
        ],
        guess_scope: StudyGuessScope::FullCanons,
        prompt_policy: PromptPolicy::WholePassage,
    }
}

fn book_of_mormon_mastery() -> StudySet {
    use Canon::BookOfMormon as BOM;
    StudySet {
        id: "doctrinal-mastery-book-of-mormon".to_string(),
        name: "Book of Mormon Doctrinal Mastery".to_string(),
        passages: vec![
            passage(BOM, "1 Nephi", 3, 7, 7),
            passage(BOM, "2 Nephi", 2, 25, 25),
            passage(BOM, "2 Nephi", 2, 27, 27),
            passage(BOM, "2 Nephi", 26, 33, 33),
            passage(BOM, "2 Nephi", 28, 30, 30),
            passage(BOM, "2 Nephi", 32, 3, 3),
            passage(BOM, "2 Nephi", 32, 8, 9),
            passage(BOM, "Mosiah", 2, 17, 17),
            passage(BOM, "Mosiah", 2, 41, 41),
            passage(BOM, "Mosiah", 3, 19, 19),
            passage(BOM, "Mosiah", 4, 9, 9),
            passage(BOM, "Mosiah", 18, 8, 10),
            passage(BOM, "Alma", 7, 11, 13),
            passage(BOM, "Alma", 34, 9, 10),
            passage(BOM, "Alma", 39, 9, 9),
            passage(BOM, "Alma", 41, 10, 10),
            passage(BOM, "Helaman", 5, 12, 12),
            passage(BOM, "3 Nephi", 11, 10, 11),
            passage(BOM, "3 Nephi", 12, 48, 48),
            passage(BOM, "3 Nephi", 27, 20, 20),
            passage(BOM, "Ether", 12, 6, 6),
            passage(BOM, "Ether", 12, 27, 27),
            passage(BOM, "Moroni", 7, 45, 48),
            passage(BOM, "Moroni", 10, 4, 5),
        ],
        guess_scope: StudyGuessScope::FullCanons,
        prompt_policy: PromptPolicy::WholePassage,
    }
}

fn doctrine_and_covenants_mastery() -> StudySet {
    use Canon::{DoctrineAndCovenants as DC, PearlOfGreatPrice as PGP};
    StudySet {
        id: "doctrinal-mastery-doctrine-and-covenants".to_string(),
        name: "Doctrine and Covenants Doctrinal Mastery".to_string(),
        passages: vec![
            passage(PGP, "Joseph Smith—History", 1, 15, 20),
            passage(DC, "D&C", 1, 30, 30),
            passage(DC, "D&C", 1, 37, 38),
            passage(DC, "D&C", 6, 36, 36),
            passage(DC, "D&C", 8, 2, 3),
            passage(DC, "D&C", 13, 1, 1),
            passage(DC, "D&C", 18, 10, 11),
            passage(DC, "D&C", 18, 15, 16),
            passage(DC, "D&C", 19, 16, 19),
            passage(DC, "D&C", 21, 4, 6),
            passage(DC, "D&C", 29, 10, 11),
            passage(DC, "D&C", 49, 15, 17),
            passage(DC, "D&C", 58, 42, 43),
            passage(DC, "D&C", 64, 9, 11),
            passage(DC, "D&C", 76, 22, 24),
            passage(DC, "D&C", 82, 10, 10),
            passage(DC, "D&C", 84, 20, 22),
            passage(DC, "D&C", 88, 118, 118),
            passage(DC, "D&C", 89, 18, 21),
            passage(DC, "D&C", 107, 8, 8),
            selected_passage(DC, "D&C", 121, &[36, 41, 42]),
            passage(DC, "D&C", 130, 22, 23),
            passage(DC, "D&C", 131, 1, 4),
            passage(DC, "D&C", 135, 3, 3),
        ],
        guess_scope: StudyGuessScope::Custom(GameScope {
            canons: vec![
                CanonScope {
                    canon: DC,
                    books: BookScope::All,
                },
                CanonScope {
                    canon: PGP,
                    books: BookScope::Selected(vec!["Joseph Smith—History".to_string()]),
                },
            ],
        }),
        prompt_policy: PromptPolicy::WholePassage,
    }
}

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

#[cfg(target_arch = "wasm32")]
fn load_custom_study_sets() -> Option<CustomStudySets> {
    let storage = web_sys::window()?.local_storage().ok()??;
    let json = storage.get_item(STORAGE_KEY).ok()??;
    serde_json::from_str(&json).ok()
}

#[cfg(not(target_arch = "wasm32"))]
fn load_custom_study_sets() -> Option<CustomStudySets> {
    None
}

#[cfg(target_arch = "wasm32")]
fn save_custom_study_sets(sets: &CustomStudySets) {
    let Some(storage) = web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    else {
        return;
    };
    let Ok(json) = serde_json::to_string(sets) else {
        return;
    };
    let _ = storage.set_item(STORAGE_KEY, &json);
}

#[cfg(not(target_arch = "wasm32"))]
fn save_custom_study_sets(_sets: &CustomStudySets) {}

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
    fn book_of_mormon_mastery_uses_the_full_canon_for_guesses() {
        let set = built_in_study_sets()
            .into_iter()
            .find(|set| set.id == "doctrinal-mastery-book-of-mormon")
            .unwrap();
        let scope = set.resolved_guess_scope();

        assert!(scope.includes_book(Canon::BookOfMormon, "Jacob"));
        assert!(!scope.includes_book(Canon::NewTestament, "Matthew"));
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
