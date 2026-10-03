use std::collections::BTreeMap;

use serde::Deserialize;

use super::playability::playable_indices;
use super::{BookInfo, BookScope, Canon, CanonScope, ChapterRef, Difficulty, GameScope, Reference};
use crate::api::{CanonMetadata, ChapterVerse, ScopeSummary};
use crate::scoring::Score;

const STANDARD_WORKS: [(Canon, &str); 5] = [
    (
        Canon::OldTestament,
        include_str!("../../assets/data/old-testament-flat.json"),
    ),
    (
        Canon::NewTestament,
        include_str!("../../assets/data/new-testament-flat.json"),
    ),
    (
        Canon::BookOfMormon,
        include_str!("../../assets/data/book-of-mormon-flat.json"),
    ),
    (
        Canon::DoctrineAndCovenants,
        include_str!("../../assets/data/doctrine-and-covenants-flat.json"),
    ),
    (
        Canon::PearlOfGreatPrice,
        include_str!("../../assets/data/pearl-of-great-price-flat.json"),
    ),
];

#[derive(Default)]
pub struct ScriptureLibrary {
    canons: BTreeMap<Canon, Scriptures>,
}

impl ScriptureLibrary {
    pub fn standard_works() -> Result<Self, serde_json::Error> {
        let mut library = Self::default();
        for (canon, data) in STANDARD_WORKS {
            library.insert(canon, Scriptures::from_flat_json_for_canon(canon, data)?);
        }
        Ok(library)
    }

    pub fn insert(&mut self, canon: Canon, scriptures: Scriptures) {
        self.canons.insert(canon, scriptures);
    }

    pub fn scriptures(&self, canon: Canon) -> Option<&Scriptures> {
        self.canons.get(&canon)
    }

    pub fn summary(&self, difficulty: Difficulty, scope: GameScope) -> ScopeSummary {
        let metadata = self
            .scoped(&scope)
            .map(|(scriptures, canon_scope)| CanonMetadata {
                canon: canon_scope.canon,
                books: scriptures.books.clone(),
                playable_verse_count: scriptures
                    .scoped_playable_verses(difficulty, &canon_scope.books)
                    .count(),
                total_verse_count: scriptures.scoped_total_verse_count(&canon_scope.books),
            })
            .collect::<Vec<_>>();
        ScopeSummary {
            difficulty,
            playable_verse_count: metadata.iter().map(|item| item.playable_verse_count).sum(),
            total_verse_count: metadata.iter().map(|item| item.total_verse_count).sum(),
            metadata,
            scope,
        }
    }

    pub fn playable_verse_count(&self, difficulty: Difficulty, scope: &GameScope) -> usize {
        self.scoped(scope)
            .map(|(scriptures, canon_scope)| {
                scriptures
                    .scoped_playable_verses(difficulty, &canon_scope.books)
                    .count()
            })
            .sum()
    }

    /// Indexes the playable verses of every canon in `scope` as one list.
    pub fn nth_playable_verse<'a>(
        &'a self,
        difficulty: Difficulty,
        scope: &'a GameScope,
        mut index: usize,
    ) -> Option<&'a Verse> {
        for (scriptures, canon_scope) in self.scoped(scope) {
            let mut verses = scriptures.scoped_playable_verses(difficulty, &canon_scope.books);
            let count = verses.clone().count();
            if index < count {
                return verses.nth(index);
            }
            index -= count;
        }
        None
    }

    pub fn chapter_verses(&self, chapter: &ChapterRef) -> Vec<ChapterVerse> {
        self.scriptures(chapter.canon)
            .map(|scriptures| {
                scriptures
                    .verses_for_chapter(&chapter.book, chapter.chapter)
                    .map(|verse| ChapterVerse {
                        verse: verse.reference.verse,
                        text: verse.text.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Scores by distance through the scope's chapters laid end to end.
    pub fn score(&self, scope: &GameScope, answer: &ChapterRef, guess: &ChapterRef) -> Score {
        let distance = if answer.canon == guess.canon {
            self.chapter_index(scope, answer)
                .zip(self.chapter_index(scope, guess))
                .map(|(answer, guess)| answer.abs_diff(guess) as u32)
        } else {
            None
        };
        Score::from_chapter_distance(distance.unwrap_or(u32::MAX))
    }

    fn scoped<'a>(
        &'a self,
        scope: &'a GameScope,
    ) -> impl Iterator<Item = (&'a Scriptures, &'a CanonScope)> {
        scope.canons.iter().filter_map(|canon_scope| {
            self.scriptures(canon_scope.canon)
                .map(|scriptures| (scriptures, canon_scope))
        })
    }

    fn chapter_index(&self, scope: &GameScope, chapter: &ChapterRef) -> Option<usize> {
        let mut offset = 0;
        for (scriptures, canon_scope) in self.scoped(scope) {
            if canon_scope.canon == chapter.canon {
                return scriptures
                    .chapter_index_in_scope(&canon_scope.books, &chapter.book, chapter.chapter)
                    .map(|index| offset + index);
            }
            offset += scriptures.chapter_count_for_scope(&canon_scope.books);
        }
        None
    }
}

pub struct Scriptures {
    pub verses: Vec<Verse>,
    pub books: Vec<BookInfo>,
    easy_verses: Vec<usize>,
    normal_verses: Vec<usize>,
    hard_verses: Vec<usize>,
    chapter_order: Vec<ChapterKey>,
}

impl Scriptures {
    pub fn from_flat_json_for_canon(canon: Canon, data: &str) -> Result<Self, serde_json::Error> {
        let flat: FlatScriptures = serde_json::from_str(data)?;
        Ok(Self::from_flat_verses(canon, flat.verses))
    }

    pub fn verses_for_chapter(&self, book: &str, chapter: u16) -> impl Iterator<Item = &Verse> {
        self.verses
            .iter()
            .filter(move |verse| verse.reference.book == book && verse.reference.chapter == chapter)
    }

    /// Joins the given verses of one chapter into a single verse.
    pub fn passage(&self, book: &str, chapter: u16, verses: &[u16]) -> Option<Verse> {
        let chapter_verses = self.verses_for_chapter(book, chapter).collect::<Vec<_>>();
        let selected = verses
            .iter()
            .map(|number| {
                chapter_verses
                    .iter()
                    .find(|verse| verse.reference.verse == *number)
                    .copied()
            })
            .collect::<Option<Vec<_>>>()?;
        let first = selected.first()?;
        Some(Verse {
            reference: first.reference.clone(),
            text: selected
                .iter()
                .map(|verse| verse.text.as_str())
                .collect::<Vec<_>>()
                .join(" "),
        })
    }

    pub fn contains_passage(&self, book: &str, chapter: u16, verses: &[u16]) -> bool {
        let Some(max_verse) = self
            .books
            .iter()
            .find(|candidate| candidate.name == book)
            .and_then(|book| book.verse_count(chapter))
        else {
            return false;
        };
        !verses.is_empty() && verses.iter().all(|verse| *verse > 0 && *verse <= max_verse)
    }

    pub fn scoped_playable_verses<'a>(
        &'a self,
        difficulty: Difficulty,
        scope: &'a BookScope,
    ) -> impl Iterator<Item = &'a Verse> + Clone {
        let indices = match difficulty {
            Difficulty::Easy => &self.easy_verses,
            Difficulty::Normal => &self.normal_verses,
            Difficulty::Hard => &self.hard_verses,
        };
        indices
            .iter()
            .map(|&index| &self.verses[index])
            .filter(|verse| scope.includes(&verse.reference.book))
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
        let mut chapter_order = Vec::<ChapterKey>::new();

        for item in flat_verses {
            let Some(reference) = parse_reference(canon, &item.reference) else {
                continue;
            };

            if books.last().map(|book| &book.name) != Some(&reference.book) {
                books.push(BookInfo {
                    name: reference.book.clone(),
                    chapters: Vec::new(),
                    chapter_verse_counts: Vec::new(),
                });
            }

            let book = books.last_mut().expect("book was just inserted if missing");
            if book.chapters.last() != Some(&reference.chapter) {
                book.chapters.push(reference.chapter);
                book.chapter_verse_counts.push(0);
                chapter_order.push(ChapterKey {
                    book: reference.book.clone(),
                    chapter: reference.chapter,
                });
            }
            if let Some(count) = book.chapter_verse_counts.last_mut() {
                *count = (*count).max(reference.verse);
            }

            verses.push(Verse {
                reference,
                text: item.text,
            });
        }

        Self {
            easy_verses: playable_indices(&verses, Difficulty::Easy),
            normal_verses: playable_indices(&verses, Difficulty::Normal),
            hard_verses: playable_indices(&verses, Difficulty::Hard),
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

#[derive(Clone, Debug, PartialEq)]
pub struct Verse {
    pub reference: Reference,
    pub text: String,
}

struct ChapterKey {
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
    use crate::scriptures::GameMode;

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

    fn chapter(canon: Canon, book: &str, chapter: u16) -> ChapterRef {
        ChapterRef {
            canon,
            book: book.to_string(),
            chapter,
        }
    }

    fn playable_count(scriptures: &Scriptures, difficulty: Difficulty) -> usize {
        scriptures
            .scoped_playable_verses(difficulty, &BookScope::All)
            .count()
    }

    #[test]
    fn builds_books_and_chapters_in_scripture_order() {
        let scriptures = sample_scriptures();

        assert_eq!(scriptures.books.len(), 2);
        assert_eq!(scriptures.books[0].name, "1 Nephi");
        assert_eq!(scriptures.books[0].chapters, vec![1, 2]);
        assert_eq!(scriptures.books[1].name, "2 Nephi");
        assert_eq!(scriptures.books[1].chapters, vec![1, 2]);
        assert_eq!(scriptures.books[1].verse_count(1), Some(1));
        assert_eq!(scriptures.books[1].verse_count(2), Some(1));
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
        assert_eq!(playable_count(&scriptures, Difficulty::Easy), 1);
        assert_eq!(playable_count(&scriptures, Difficulty::Normal), 1);
        assert_eq!(playable_count(&scriptures, Difficulty::Hard), 2);
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

        assert_eq!(playable_count(&scriptures, Difficulty::Easy), 2);
        assert_eq!(playable_count(&scriptures, Difficulty::Normal), 2);
        assert_eq!(playable_count(&scriptures, Difficulty::Hard), 2);
    }

    #[test]
    fn scores_across_book_boundaries_by_flattened_chapter_distance() {
        let score = sample_library().score(
            &GameMode::BookOfMormon.scope(),
            &chapter(Canon::BookOfMormon, "1 Nephi", 2),
            &chapter(Canon::BookOfMormon, "2 Nephi", 1),
        );

        assert_eq!(score.chapter_distance, 1);
        assert_eq!(score.points, 978);
    }

    #[test]
    fn unknown_guess_scores_zero() {
        let score = sample_library().score(
            &GameMode::BookOfMormon.scope(),
            &chapter(Canon::BookOfMormon, "1 Nephi", 1),
            &chapter(Canon::BookOfMormon, "Jacob", 1),
        );

        assert_eq!(score.points, 0);
    }

    #[test]
    fn finds_verses_for_containing_chapter() {
        let scriptures = sample_scriptures();

        let verses = scriptures
            .verses_for_chapter("1 Nephi", 1)
            .collect::<Vec<_>>();

        assert_eq!(verses.len(), 2);
        assert_eq!(verses[0].reference.verse, 1);
        assert_eq!(verses[1].reference.verse, 2);
    }

    #[test]
    fn joins_passage_verses_in_request_order() {
        let passage = sample_scriptures().passage("1 Nephi", 1, &[1, 2]).unwrap();

        assert_eq!(passage.reference.verse, 1);
        assert_eq!(passage.text, "First verse Second verse");
        assert!(sample_scriptures().passage("1 Nephi", 1, &[3]).is_none());
    }

    #[test]
    fn guesses_in_the_wrong_canon_score_zero() {
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

        let score = library.score(
            &GameMode::Bible.scope(),
            &chapter(Canon::OldTestament, "Genesis", 2),
            &chapter(Canon::NewTestament, "Matthew", 1),
        );

        assert_eq!(score.chapter_distance, u32::MAX);
        assert_eq!(score.points, 0);
    }

    #[test]
    fn scoped_verses_are_filtered_to_selected_books() {
        let scriptures = sample_scriptures();
        let scope = BookScope::Selected(vec!["2 Nephi".to_string()]);

        let verses = scriptures
            .scoped_playable_verses(Difficulty::Hard, &scope)
            .collect::<Vec<_>>();

        assert_eq!(verses.len(), 2);
        assert!(verses.iter().all(|verse| verse.reference.book == "2 Nephi"));
    }

    #[test]
    fn scoped_scoring_skips_unselected_books() {
        let scope = GameScope {
            canons: vec![CanonScope {
                canon: Canon::BookOfMormon,
                books: BookScope::Selected(vec!["1 Nephi".to_string(), "2 Nephi".to_string()]),
            }],
        };

        let score = sample_library().score(
            &scope,
            &chapter(Canon::BookOfMormon, "1 Nephi", 2),
            &chapter(Canon::BookOfMormon, "2 Nephi", 1),
        );

        assert_eq!(score.chapter_distance, 1);
    }

    #[test]
    fn indexes_playable_verses_across_canons() {
        let library = ScriptureLibrary::standard_works().unwrap();
        let scope = GameMode::Bible.scope();
        let old_testament = library
            .scriptures(Canon::OldTestament)
            .unwrap()
            .scoped_playable_verses(Difficulty::Easy, &BookScope::All)
            .count();

        let first_new_testament = library
            .nth_playable_verse(Difficulty::Easy, &scope, old_testament)
            .unwrap();

        assert_eq!(first_new_testament.reference.canon, Canon::NewTestament);
        assert_eq!(
            library.playable_verse_count(Difficulty::Easy, &scope),
            library
                .summary(Difficulty::Easy, scope)
                .playable_verse_count
        );
    }
}
