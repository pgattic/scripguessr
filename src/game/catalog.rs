use crate::api::CanonMetadata;
use crate::scriptures::{BookInfo, Canon, GameScope};

pub fn books(metadata: &[CanonMetadata], canon: Canon) -> &[BookInfo] {
    metadata
        .iter()
        .find(|item| item.canon == canon)
        .map(|item| item.books.as_slice())
        .unwrap_or_default()
}

pub fn books_in_scope(
    metadata: &[CanonMetadata],
    scope: &GameScope,
    canon: Canon,
) -> Vec<BookInfo> {
    let book_scope = scope.canon_scope(canon).map(|scope| &scope.books);
    books(metadata, canon)
        .iter()
        .filter(|book| book_scope.is_none_or(|books| books.includes(&book.name)))
        .cloned()
        .collect()
}

pub fn chapters(metadata: &[CanonMetadata], canon: Canon, book: &str) -> Vec<u16> {
    books(metadata, canon)
        .iter()
        .find(|candidate| candidate.name == book)
        .map(|book| book.chapters.clone())
        .unwrap_or_default()
}
