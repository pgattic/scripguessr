#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::scoring::Score;

#[derive(Clone, Default, PartialEq)]
pub struct ScriptureLibrary {
    canons: BTreeMap<Canon, Arc<Scriptures>>,
}

impl ScriptureLibrary {
    pub fn insert(&mut self, canon: Canon, scriptures: Scriptures) {
        self.canons.insert(canon, Arc::new(scriptures));
    }

    pub fn scriptures(&self, canon: Canon) -> Option<&Scriptures> {
        self.canons.get(&canon).map(Arc::as_ref)
    }

    pub fn score(
        &self,
        scope: &GameScope,
        answer: &Reference,
        guess_canon: Canon,
        guess_book: &str,
        guess_chapter: u16,
    ) -> Score {
        let Some(answer_index) =
            self.chapter_index(scope, answer.canon, &answer.book, answer.chapter)
        else {
            return Score::from_chapter_distance(u32::MAX);
        };
        let Some(guess_index) = self.chapter_index(scope, guess_canon, guess_book, guess_chapter)
        else {
            return Score::from_chapter_distance(u32::MAX);
        };

        Score::from_chapter_distance(answer_index.abs_diff(guess_index) as u32)
    }

    fn chapter_index(
        &self,
        scope: &GameScope,
        canon: Canon,
        book: &str,
        chapter: u16,
    ) -> Option<usize> {
        let mut offset = 0;

        for canon_scope in &scope.canons {
            let scriptures = self.scriptures(canon_scope.canon)?;
            if canon_scope.canon == canon {
                return scriptures
                    .chapter_index_in_scope(&canon_scope.books, book, chapter)
                    .map(|index| offset + index);
            }

            offset += scriptures.chapter_count_for_scope(&canon_scope.books);
        }

        None
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Canon {
    BookOfMormon,
    DoctrineAndCovenants,
    PearlOfGreatPrice,
    OldTestament,
    NewTestament,
}

impl Canon {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub const ALL: [Self; 5] = [
        Self::OldTestament,
        Self::NewTestament,
        Self::BookOfMormon,
        Self::DoctrineAndCovenants,
        Self::PearlOfGreatPrice,
    ];

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn label(self) -> &'static str {
        match self {
            Self::BookOfMormon => "Book of Mormon",
            Self::DoctrineAndCovenants => "Doctrine and Covenants",
            Self::PearlOfGreatPrice => "Pearl of Great Price",
            Self::OldTestament => "Old Testament",
            Self::NewTestament => "New Testament",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub enum GameMode {
    BookOfMormon,
    Bible,
    Restoration,
    AllStandardWorks,
}

impl GameMode {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub const ALL: [Self; 4] = [
        Self::BookOfMormon,
        Self::Bible,
        Self::Restoration,
        Self::AllStandardWorks,
    ];

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn label(self) -> &'static str {
        match self {
            Self::BookOfMormon => "Book of Mormon",
            Self::Bible => "Bible",
            Self::Restoration => "Restoration",
            Self::AllStandardWorks => "All Standard Works",
        }
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn canons(self) -> &'static [Canon] {
        match self {
            Self::BookOfMormon => &[Canon::BookOfMormon],
            Self::Bible => &[Canon::OldTestament, Canon::NewTestament],
            Self::Restoration => &[
                Canon::BookOfMormon,
                Canon::DoctrineAndCovenants,
                Canon::PearlOfGreatPrice,
            ],
            Self::AllStandardWorks => &[
                Canon::OldTestament,
                Canon::NewTestament,
                Canon::BookOfMormon,
                Canon::DoctrineAndCovenants,
                Canon::PearlOfGreatPrice,
            ],
        }
    }

    pub fn scope(self) -> GameScope {
        GameScope {
            canons: self
                .canons()
                .iter()
                .copied()
                .map(|canon| CanonScope {
                    canon,
                    books: BookScope::All,
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct GameScope {
    pub canons: Vec<CanonScope>,
}

impl GameScope {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn canons(&self) -> Vec<Canon> {
        self.canons.iter().map(|scope| scope.canon).collect()
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn contains_canon(&self, canon: Canon) -> bool {
        self.canons.iter().any(|scope| scope.canon == canon)
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn canon_scope(&self, canon: Canon) -> Option<&CanonScope> {
        self.canons.iter().find(|scope| scope.canon == canon)
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn selected_book_names(&self, canon: Canon) -> Option<Vec<String>> {
        let canon_scope = self.canon_scope(canon)?;
        match &canon_scope.books {
            BookScope::All => None,
            BookScope::Selected(books) => Some(books.clone()),
        }
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn includes_book(&self, canon: Canon, book: &str) -> bool {
        self.canon_scope(canon)
            .map(|scope| scope.books.includes(book))
            .unwrap_or(false)
    }
}

impl Default for GameScope {
    fn default() -> Self {
        GameMode::BookOfMormon.scope()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct CanonScope {
    pub canon: Canon,
    pub books: BookScope,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum BookScope {
    All,
    Selected(Vec<String>),
}

impl BookScope {
    pub fn includes(&self, book: &str) -> bool {
        match self {
            Self::All => true,
            Self::Selected(books) => books.iter().any(|selected| selected == book),
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct Scriptures {
    pub verses: Vec<Verse>,
    pub books: Vec<BookInfo>,
    easy_verses: Vec<Verse>,
    normal_verses: Vec<Verse>,
    hard_verses: Vec<Verse>,
    chapter_order: Vec<ChapterRef>,
}

impl Scriptures {
    pub fn from_flat_json_for_canon(canon: Canon, data: &str) -> Result<Self, serde_json::Error> {
        let flat: FlatScriptures = serde_json::from_str(data)?;
        Ok(Self::from_flat_verses(canon, flat.verses))
    }

    pub fn verses_for_chapter(&self, reference: &Reference) -> Vec<Verse> {
        self.verses
            .iter()
            .filter(|verse| {
                verse.reference.canon == reference.canon
                    && verse.reference.book == reference.book
                    && verse.reference.chapter == reference.chapter
            })
            .cloned()
            .collect()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn verse_count_for_difficulty(&self, difficulty: Difficulty) -> usize {
        self.verses_for_difficulty(difficulty).len()
    }

    pub fn verses_for_difficulty(&self, difficulty: Difficulty) -> &[Verse] {
        match difficulty {
            Difficulty::Easy => &self.easy_verses,
            Difficulty::Normal => &self.normal_verses,
            Difficulty::Hard => &self.hard_verses,
        }
    }

    pub fn scoped_verses_for_difficulty(
        &self,
        difficulty: Difficulty,
        scope: &BookScope,
    ) -> Vec<Verse> {
        self.verses_for_difficulty(difficulty)
            .iter()
            .filter(|verse| scope.includes(&verse.reference.book))
            .cloned()
            .collect()
    }

    pub fn scoped_verse_count_for_difficulty(
        &self,
        difficulty: Difficulty,
        scope: &BookScope,
    ) -> usize {
        self.verses_for_difficulty(difficulty)
            .iter()
            .filter(|verse| scope.includes(&verse.reference.book))
            .count()
    }

    pub fn scoped_total_verse_count(&self, scope: &BookScope) -> usize {
        self.verses
            .iter()
            .filter(|verse| scope.includes(&verse.reference.book))
            .count()
    }

    fn from_flat_verses(canon: Canon, flat_verses: Vec<FlatVerse>) -> Self {
        let mut verses = Vec::new();
        let mut books = Vec::<BookInfo>::new();
        let mut chapter_order = Vec::<ChapterRef>::new();

        for item in flat_verses {
            let Some(reference) = parse_reference(canon, &item.reference) else {
                continue;
            };

            if books.last().map(|book| &book.name) != Some(&reference.book) {
                books.push(BookInfo {
                    name: reference.book.clone(),
                    chapters: Vec::new(),
                });
            }

            let book = books.last_mut().expect("book was just inserted if missing");
            if book.chapters.last() != Some(&reference.chapter) {
                book.chapters.push(reference.chapter);
                chapter_order.push(ChapterRef {
                    book: reference.book.clone(),
                    chapter: reference.chapter,
                });
            }

            verses.push(Verse {
                reference,
                text: item.text,
            });
        }

        Self {
            easy_verses: filtered_verses(&verses, Difficulty::Easy),
            normal_verses: filtered_verses(&verses, Difficulty::Normal),
            hard_verses: filtered_verses(&verses, Difficulty::Hard),
            verses,
            books,
            chapter_order,
        }
    }

    fn chapter_index_in_scope(&self, scope: &BookScope, book: &str, chapter: u16) -> Option<usize> {
        self.chapter_order
            .iter()
            .filter(|item| scope.includes(&item.book))
            .position(|item| item.book == book && item.chapter == chapter)
    }

    fn chapter_count_for_scope(&self, scope: &BookScope) -> usize {
        self.chapter_order
            .iter()
            .filter(|item| scope.includes(&item.book))
            .count()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Difficulty {
    Easy,
    Normal,
    Hard,
}

impl Difficulty {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub const ALL: [Self; 3] = [Self::Easy, Self::Normal, Self::Hard];

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn label(self) -> &'static str {
        match self {
            Self::Easy => "Easy",
            Self::Normal => "Normal",
            Self::Hard => "Hard",
        }
    }
}

fn filtered_verses(verses: &[Verse], difficulty: Difficulty) -> Vec<Verse> {
    let filtered = verses
        .iter()
        .filter(|verse| is_playable_verse(&verse.text, difficulty))
        .cloned()
        .collect::<Vec<_>>();

    if filtered.is_empty() {
        verses.to_vec()
    } else {
        filtered
    }
}

fn is_playable_verse(text: &str, difficulty: Difficulty) -> bool {
    let normalized_words = normalized_words(text);
    let word_count = normalized_words.len();
    let minimum_words = match difficulty {
        Difficulty::Easy => 24,
        Difficulty::Normal => 16,
        Difficulty::Hard => 8,
    };

    if word_count < minimum_words {
        return false;
    }

    let distinct_words = normalized_words
        .iter()
        .collect::<std::collections::BTreeSet<_>>()
        .len();

    let minimum_distinct_words = match difficulty {
        Difficulty::Easy => 16,
        Difficulty::Normal => 10,
        Difficulty::Hard => 6,
    };

    if distinct_words < minimum_distinct_words {
        return false;
    }

    let lower = text.to_lowercase();
    let filler_hits = [
        "and it came to pass",
        "now behold",
        "and now",
        "yea",
        "therefore",
    ]
    .iter()
    .filter(|phrase| lower.matches(*phrase).count() > 0)
    .count();

    let distinct_ratio = distinct_words as f32 / word_count as f32;
    match difficulty {
        Difficulty::Easy => filler_hits <= 1 || distinct_ratio >= 0.8,
        Difficulty::Normal => filler_hits <= 2 || distinct_ratio >= 0.72,
        Difficulty::Hard => filler_hits <= 2 || distinct_ratio >= 0.62,
    }
}

fn normalized_words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|word| {
            word.chars()
                .filter(|character| character.is_alphanumeric() || *character == '\'')
                .collect::<String>()
                .to_lowercase()
        })
        .filter(|word| !word.is_empty())
        .collect()
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct BookInfo {
    pub name: String,
    pub chapters: Vec<u16>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Verse {
    pub reference: Reference,
    pub text: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Reference {
    pub canon: Canon,
    pub book: String,
    pub chapter: u16,
    pub verse: u16,
}

#[derive(Clone, PartialEq)]
struct ChapterRef {
    book: String,
    chapter: u16,
}

#[derive(Deserialize)]
struct FlatScriptures {
    verses: Vec<FlatVerse>,
}

#[derive(Deserialize)]
struct FlatVerse {
    reference: String,
    text: String,
}

fn parse_reference(canon: Canon, reference: &str) -> Option<Reference> {
    let (book_and_chapter, verse) = reference.rsplit_once(':')?;
    let last_space = book_and_chapter.rfind(' ')?;
    let (book, chapter) = book_and_chapter.split_at(last_space);
    let chapter = chapter.trim().parse().ok()?;
    let verse = verse.parse().ok()?;

    Some(Reference {
        canon,
        book: book.to_string(),
        chapter,
        verse,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_DATA: &str = r#"{
      "verses": [
        { "reference": "1 Nephi 1:1", "text": "First verse" },
        { "reference": "1 Nephi 1:2", "text": "Second verse" },
        { "reference": "1 Nephi 2:1", "text": "Third verse" },
        { "reference": "2 Nephi 1:1", "text": "Fourth verse" },
        { "reference": "2 Nephi 2:1", "text": "Fifth verse" }
      ]
    }"#;

    fn sample_scriptures() -> Scriptures {
        Scriptures::from_flat_json_for_canon(Canon::BookOfMormon, SAMPLE_DATA).unwrap()
    }

    fn sample_library() -> ScriptureLibrary {
        let mut library = ScriptureLibrary::default();
        library.insert(Canon::BookOfMormon, sample_scriptures());
        library
    }

    #[test]
    fn builds_books_and_chapters_in_scripture_order() {
        let scriptures = sample_scriptures();

        assert_eq!(scriptures.books.len(), 2);
        assert_eq!(scriptures.books[0].name, "1 Nephi");
        assert_eq!(scriptures.books[0].chapters, vec![1, 2]);
        assert_eq!(scriptures.books[1].name, "2 Nephi");
        assert_eq!(scriptures.books[1].chapters, vec![1, 2]);
        assert_eq!(scriptures.verses.len(), 5);
    }

    #[test]
    fn builds_filtered_verse_pools_by_difficulty() {
        let scriptures = Scriptures::from_flat_json_for_canon(
            Canon::BookOfMormon,
            r#"{
              "verses": [
                { "reference": "1 Nephi 1:1", "text": "Amen." },
                { "reference": "1 Nephi 1:2", "text": "Nephi keeps a record about his family, their journey, and the mercies of the Lord." },
                { "reference": "1 Nephi 1:2", "text": "And it came to pass, yea, and now therefore it came to pass, yea, and now therefore it came to pass again." },
                { "reference": "1 Nephi 1:3", "text": "Nephi records the learning of his father, the mercy of God, and the purposes that shaped his journey through the wilderness with faith and patience." }
              ]
            }"#,
        )
        .unwrap();

        assert_eq!(scriptures.verses.len(), 4);
        assert_eq!(scriptures.verse_count_for_difficulty(Difficulty::Easy), 1);
        assert_eq!(scriptures.verse_count_for_difficulty(Difficulty::Normal), 1);
        assert_eq!(scriptures.verse_count_for_difficulty(Difficulty::Hard), 2);
    }

    #[test]
    fn falls_back_to_all_verses_if_filter_removes_everything() {
        let scriptures = Scriptures::from_flat_json_for_canon(
            Canon::BookOfMormon,
            r#"{
              "verses": [
                { "reference": "1 Nephi 1:1", "text": "Amen." },
                { "reference": "1 Nephi 1:2", "text": "Yea." }
              ]
            }"#,
        )
        .unwrap();

        assert_eq!(scriptures.verse_count_for_difficulty(Difficulty::Easy), 2);
        assert_eq!(scriptures.verse_count_for_difficulty(Difficulty::Normal), 2);
        assert_eq!(scriptures.verse_count_for_difficulty(Difficulty::Hard), 2);
    }

    #[test]
    fn scores_across_book_boundaries_by_flattened_chapter_distance() {
        let library = sample_library();
        let answer = Reference {
            canon: Canon::BookOfMormon,
            book: "1 Nephi".to_string(),
            chapter: 2,
            verse: 1,
        };

        let score = library.score(
            &GameMode::BookOfMormon.scope(),
            &answer,
            Canon::BookOfMormon,
            "2 Nephi",
            1,
        );

        assert_eq!(score.chapter_distance, 1);
        assert_eq!(score.points, 978);
    }

    #[test]
    fn unknown_guess_scores_zero() {
        let library = sample_library();
        let answer = Reference {
            canon: Canon::BookOfMormon,
            book: "1 Nephi".to_string(),
            chapter: 1,
            verse: 1,
        };

        assert_eq!(
            library
                .score(
                    &GameMode::BookOfMormon.scope(),
                    &answer,
                    Canon::BookOfMormon,
                    "Jacob",
                    1,
                )
                .points,
            0
        );
    }

    #[test]
    fn finds_verses_for_containing_chapter() {
        let scriptures = sample_scriptures();
        let reference = Reference {
            canon: Canon::BookOfMormon,
            book: "1 Nephi".to_string(),
            chapter: 1,
            verse: 1,
        };

        let verses = scriptures.verses_for_chapter(&reference);

        assert_eq!(verses.len(), 2);
        assert_eq!(verses[0].reference.verse, 1);
        assert_eq!(verses[1].reference.verse, 2);
    }

    #[test]
    fn library_scores_across_loaded_canons_in_mode_order() {
        let mut library = ScriptureLibrary::default();
        library.insert(
            Canon::OldTestament,
            Scriptures::from_flat_json_for_canon(
                Canon::OldTestament,
                r#"{
                  "verses": [
                    { "reference": "Genesis 1:1", "text": "First verse" },
                    { "reference": "Genesis 2:1", "text": "Second verse" }
                  ]
                }"#,
            )
            .unwrap(),
        );
        library.insert(
            Canon::NewTestament,
            Scriptures::from_flat_json_for_canon(
                Canon::NewTestament,
                r#"{
                  "verses": [
                    { "reference": "Matthew 1:1", "text": "Third verse" }
                  ]
                }"#,
            )
            .unwrap(),
        );
        let answer = Reference {
            canon: Canon::OldTestament,
            book: "Genesis".to_string(),
            chapter: 2,
            verse: 1,
        };

        let score = library.score(
            &GameMode::Bible.scope(),
            &answer,
            Canon::NewTestament,
            "Matthew",
            1,
        );

        assert_eq!(score.chapter_distance, 1);
        assert_eq!(score.points, 978);
    }

    #[test]
    fn scoped_verses_are_filtered_to_selected_books() {
        let scriptures = sample_scriptures();
        let scope = BookScope::Selected(vec!["2 Nephi".to_string()]);

        let verses = scriptures.scoped_verses_for_difficulty(Difficulty::Hard, &scope);

        assert_eq!(verses.len(), 2);
        assert!(verses.iter().all(|verse| verse.reference.book == "2 Nephi"));
    }

    #[test]
    fn scoped_scoring_skips_unselected_books() {
        let library = sample_library();
        let scope = GameScope {
            canons: vec![CanonScope {
                canon: Canon::BookOfMormon,
                books: BookScope::Selected(vec!["1 Nephi".to_string(), "2 Nephi".to_string()]),
            }],
        };
        let answer = Reference {
            canon: Canon::BookOfMormon,
            book: "1 Nephi".to_string(),
            chapter: 2,
            verse: 1,
        };

        let score = library.score(&scope, &answer, Canon::BookOfMormon, "2 Nephi", 1);

        assert_eq!(score.chapter_distance, 1);
    }
}
