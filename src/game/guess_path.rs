use super::catalog::{books_in_scope, chapters};
use crate::api::CanonMetadata;
use crate::scriptures::{Canon, ChapterRef, GameScope};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum GuessStep {
    #[default]
    Canon,
    Book,
    Chapter,
    Ready,
}

impl GuessStep {
    pub fn label(self) -> &'static str {
        match self {
            Self::Canon => "Canon",
            Self::Book => "Book",
            Self::Chapter => "Chapter",
            Self::Ready => "Review",
        }
    }
}

/// The player's canon, book, and chapter picks for the current round.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GuessPath {
    pub canon: Option<Canon>,
    pub book: Option<String>,
    pub chapter: Option<u16>,
    pub step: GuessStep,
}

impl GuessPath {
    pub fn select_canon(&mut self, canon: Canon) {
        if self.canon != Some(canon) {
            self.book = None;
            self.chapter = None;
        }
        self.canon = Some(canon);
        self.step = GuessStep::Book;
    }

    pub fn select_book(&mut self, book: String) {
        if self.book.as_ref() != Some(&book) {
            self.chapter = None;
        }
        self.book = Some(book);
        self.step = GuessStep::Chapter;
    }

    pub fn select_chapter(&mut self, chapter: u16) {
        self.chapter = Some(chapter);
        self.step = GuessStep::Ready;
    }

    pub fn open_canon(&mut self) {
        if self.canon.is_some() {
            self.step = GuessStep::Book;
        }
    }

    pub fn open_book(&mut self) {
        if self.book.is_some() {
            self.step = GuessStep::Chapter;
        }
    }

    pub fn open_chapter(&mut self) {
        if self.chapter.is_some() {
            self.step = GuessStep::Ready;
        }
    }

    pub fn chapter_ref(&self) -> Option<ChapterRef> {
        Some(ChapterRef {
            canon: self.canon?,
            book: self.book.clone()?,
            chapter: self.chapter?,
        })
    }

    pub fn skip_single_choices(&mut self, scope: &GameScope, metadata: &[CanonMetadata]) {
        if self.step == GuessStep::Canon && scope.canons.len() == 1 {
            self.canon = scope.canons.first().map(|scope| scope.canon);
            self.step = GuessStep::Book;
        }

        if self.step == GuessStep::Book {
            let Some(canon) = self.canon else {
                return;
            };
            let books = books_in_scope(metadata, scope, canon);
            if let [book] = books.as_slice() {
                self.book = Some(book.name.clone());
                self.step = GuessStep::Chapter;
            }
        }

        if self.step == GuessStep::Chapter {
            let (Some(canon), Some(book)) = (self.canon, self.book.as_deref()) else {
                return;
            };
            if let [chapter] = chapters(metadata, canon, book).as_slice() {
                self.chapter = Some(*chapter);
                self.step = GuessStep::Ready;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scriptures::{BookInfo, GameMode};

    fn metadata() -> Vec<CanonMetadata> {
        vec![CanonMetadata {
            canon: Canon::PearlOfGreatPrice,
            books: vec![BookInfo {
                name: "Articles of Faith".to_string(),
                chapters: vec![1],
                chapter_verse_counts: vec![13],
            }],
            playable_verse_count: 13,
            total_verse_count: 13,
        }]
    }

    #[test]
    fn single_choices_are_filled_in() {
        let scope = GameScope {
            canons: vec![crate::scriptures::CanonScope {
                canon: Canon::PearlOfGreatPrice,
                books: crate::scriptures::BookScope::All,
            }],
        };
        let mut path = GuessPath::default();

        path.skip_single_choices(&scope, &metadata());

        assert_eq!(path.step, GuessStep::Ready);
        assert_eq!(
            path.chapter_ref(),
            Some(ChapterRef {
                canon: Canon::PearlOfGreatPrice,
                book: "Articles of Faith".to_string(),
                chapter: 1,
            })
        );
    }

    #[test]
    fn changing_canon_clears_later_picks() {
        let mut path = GuessPath::default();
        path.select_canon(Canon::OldTestament);
        path.select_book("Genesis".to_string());
        path.select_chapter(1);

        path.select_canon(Canon::NewTestament);

        assert_eq!(path.book, None);
        assert_eq!(path.chapter, None);
        assert_eq!(path.step, GuessStep::Book);
        let mut bible = GuessPath::default();
        bible.skip_single_choices(&GameMode::Bible.scope(), &metadata());
        assert_eq!(bible.step, GuessStep::Canon);
    }
}
