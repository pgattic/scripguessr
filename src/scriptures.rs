use serde::Deserialize;

use crate::scoring::Score;

#[derive(Clone)]
pub struct Scriptures {
    pub verses: Vec<Verse>,
    pub playable_verses: Vec<Verse>,
    pub books: Vec<BookInfo>,
    chapter_order: Vec<ChapterRef>,
}

impl Scriptures {
    pub fn from_flat_json(data: &str) -> Result<Self, serde_json::Error> {
        let flat: FlatScriptures = serde_json::from_str(data)?;
        Ok(Self::from_flat_verses(flat.verses))
    }

    pub fn score(&self, answer: &Reference, guess_book: &str, guess_chapter: u16) -> Score {
        let Some(answer_index) = self.chapter_index(&answer.book, answer.chapter) else {
            return Score::from_chapter_distance(u32::MAX);
        };
        let Some(guess_index) = self.chapter_index(guess_book, guess_chapter) else {
            return Score::from_chapter_distance(u32::MAX);
        };

        Score::from_chapter_distance(answer_index.abs_diff(guess_index) as u32)
    }

    pub fn chapters_for(&self, book: &str) -> Vec<u16> {
        self.books
            .iter()
            .find(|item| item.name == book)
            .map(|item| item.chapters.clone())
            .unwrap_or_default()
    }

    pub fn total_verse_count(&self) -> usize {
        self.verses.len()
    }

    fn from_flat_verses(flat_verses: Vec<FlatVerse>) -> Self {
        let mut verses = Vec::new();
        let mut books = Vec::<BookInfo>::new();
        let mut chapter_order = Vec::<ChapterRef>::new();

        for item in flat_verses {
            let Some(reference) = parse_reference(&item.reference) else {
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
            playable_verses: playable_verses(&verses),
            verses,
            books,
            chapter_order,
        }
    }

    fn chapter_index(&self, book: &str, chapter: u16) -> Option<usize> {
        self.chapter_order
            .iter()
            .position(|item| item.book == book && item.chapter == chapter)
    }
}

fn playable_verses(verses: &[Verse]) -> Vec<Verse> {
    let filtered = verses
        .iter()
        .filter(|verse| is_playable_verse(&verse.text))
        .cloned()
        .collect::<Vec<_>>();

    if filtered.is_empty() {
        verses.to_vec()
    } else {
        filtered
    }
}

fn is_playable_verse(text: &str) -> bool {
    let normalized_words = normalized_words(text);

    if normalized_words.len() < 16 {
        return false;
    }

    let distinct_words = normalized_words
        .iter()
        .collect::<std::collections::BTreeSet<_>>()
        .len();

    if distinct_words < 10 {
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

    let distinct_ratio = distinct_words as f32 / normalized_words.len() as f32;
    filler_hits <= 2 || distinct_ratio >= 0.72
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

#[derive(Clone, PartialEq)]
pub struct BookInfo {
    pub name: String,
    pub chapters: Vec<u16>,
}

#[derive(Clone, PartialEq)]
pub struct Verse {
    pub reference: Reference,
    pub text: String,
}

#[derive(Clone, PartialEq)]
pub struct Reference {
    pub book: String,
    pub chapter: u16,
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

fn parse_reference(reference: &str) -> Option<Reference> {
    let (book_and_chapter, _verse) = reference.rsplit_once(':')?;
    let last_space = book_and_chapter.rfind(' ')?;
    let (book, chapter) = book_and_chapter.split_at(last_space);
    let chapter = chapter.trim().parse().ok()?;

    Some(Reference {
        book: book.to_string(),
        chapter,
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

    #[test]
    fn builds_books_and_chapters_in_scripture_order() {
        let scriptures = Scriptures::from_flat_json(SAMPLE_DATA).unwrap();

        assert_eq!(scriptures.books.len(), 2);
        assert_eq!(scriptures.books[0].name, "1 Nephi");
        assert_eq!(scriptures.books[0].chapters, vec![1, 2]);
        assert_eq!(scriptures.books[1].name, "2 Nephi");
        assert_eq!(scriptures.books[1].chapters, vec![1, 2]);
        assert_eq!(scriptures.verses.len(), 5);
    }

    #[test]
    fn builds_a_filtered_playable_verse_pool() {
        let scriptures = Scriptures::from_flat_json(
            r#"{
              "verses": [
                { "reference": "1 Nephi 1:1", "text": "Amen." },
                { "reference": "1 Nephi 1:2", "text": "And it came to pass, yea, and now therefore it came to pass, yea, and now therefore it came to pass again." },
                { "reference": "1 Nephi 1:3", "text": "Nephi records the learning of his father, the mercy of God, and the purposes that shaped his journey through the wilderness." }
              ]
            }"#,
        )
        .unwrap();

        assert_eq!(scriptures.verses.len(), 3);
        assert_eq!(scriptures.playable_verses.len(), 1);
        assert_eq!(scriptures.playable_verses[0].reference.chapter, 1);
        assert_eq!(
            scriptures.playable_verses[0].text,
            "Nephi records the learning of his father, the mercy of God, and the purposes that shaped his journey through the wilderness."
        );
    }

    #[test]
    fn falls_back_to_all_verses_if_filter_removes_everything() {
        let scriptures = Scriptures::from_flat_json(
            r#"{
              "verses": [
                { "reference": "1 Nephi 1:1", "text": "Amen." },
                { "reference": "1 Nephi 1:2", "text": "Yea." }
              ]
            }"#,
        )
        .unwrap();

        assert_eq!(scriptures.playable_verses.len(), 2);
    }

    #[test]
    fn scores_across_book_boundaries_by_flattened_chapter_distance() {
        let scriptures = Scriptures::from_flat_json(SAMPLE_DATA).unwrap();
        let answer = Reference {
            book: "1 Nephi".to_string(),
            chapter: 2,
        };

        let score = scriptures.score(&answer, "2 Nephi", 1);

        assert_eq!(score.chapter_distance, 1);
        assert_eq!(score.points, 978);
    }

    #[test]
    fn unknown_guess_scores_zero() {
        let scriptures = Scriptures::from_flat_json(SAMPLE_DATA).unwrap();
        let answer = Reference {
            book: "1 Nephi".to_string(),
            chapter: 1,
        };

        assert_eq!(scriptures.score(&answer, "Jacob", 1).points, 0);
    }
}
