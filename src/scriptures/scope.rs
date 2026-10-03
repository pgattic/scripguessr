use serde::{Deserialize, Serialize};

use super::{Canon, GameMode};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct GameScope {
    pub canons: Vec<CanonScope>,
}

impl GameScope {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn contains_canon(&self, canon: Canon) -> bool {
        self.canon_scope(canon).is_some()
    }

    pub fn canon_scope(&self, canon: Canon) -> Option<&CanonScope> {
        self.canons.iter().find(|scope| scope.canon == canon)
    }

    pub fn includes_book(&self, canon: Canon, book: &str) -> bool {
        self.canon_scope(canon)
            .is_some_and(|scope| scope.books.includes(book))
    }

    /// Added canons keep `Canon::ALL` order.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn toggle_canon(&mut self, canon: Canon) {
        if self.contains_canon(canon) {
            self.canons.retain(|scope| scope.canon != canon);
        } else {
            self.canons.push(CanonScope {
                canon,
                books: BookScope::All,
            });
            self.canons.sort_by_key(|scope| scope.canon.position());
        }
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn set_books(&mut self, canon: Canon, books: BookScope) {
        if let Some(scope) = self.canon_scope_mut(canon) {
            scope.books = books;
        }
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn toggle_book(&mut self, canon: Canon, book: &str) {
        if let Some(scope) = self.canon_scope_mut(canon) {
            scope.books.toggle_book(book);
        }
    }

    fn canon_scope_mut(&mut self, canon: Canon) -> Option<&mut CanonScope> {
        self.canons.iter_mut().find(|scope| scope.canon == canon)
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

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn summary(&self) -> String {
        match self {
            Self::All => "All books".to_string(),
            Self::Selected(books) if books.len() == 1 => format!("{} only", books[0]),
            Self::Selected(books) => format!("{} books", books.len()),
        }
    }

    /// Leaves `All` unchanged.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn toggle_book(&mut self, book: &str) {
        if let Self::Selected(books) = self {
            if let Some(index) = books.iter().position(|selected| selected == book) {
                books.remove(index);
            } else {
                books.push(book.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggling_a_canon_keeps_canonical_order() {
        let mut scope = GameMode::BookOfMormon.scope();

        scope.toggle_canon(Canon::OldTestament);
        assert_eq!(scope.canons[0].canon, Canon::OldTestament);
        assert_eq!(scope.canons[1].canon, Canon::BookOfMormon);

        scope.toggle_canon(Canon::OldTestament);
        assert_eq!(scope, GameMode::BookOfMormon.scope());
    }

    #[test]
    fn toggling_a_book_only_changes_selections() {
        let mut books = BookScope::Selected(vec!["Alma".to_string()]);
        books.toggle_book("Jacob");
        assert_eq!(
            books,
            BookScope::Selected(vec!["Alma".to_string(), "Jacob".to_string()])
        );
        books.toggle_book("Alma");
        assert_eq!(books, BookScope::Selected(vec!["Jacob".to_string()]));

        let mut all = BookScope::All;
        all.toggle_book("Alma");
        assert_eq!(all, BookScope::All);
    }
}
