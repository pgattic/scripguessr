use crate::scriptures::{BookScope, CanonScope, Difficulty, GameMode, GameScope};

#[derive(Clone, Debug, PartialEq)]
pub struct GameSettings {
    pub round_count: usize,
    pub difficulty: Difficulty,
    pub scope: GameScope,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            round_count: 5,
            difficulty: Difficulty::Easy,
            scope: GameScope::default(),
        }
    }
}

impl GameSettings {
    pub fn selection_label(&self) -> String {
        if self.scope.canons.is_empty() {
            return "No scope selected".to_string();
        }

        if let Some(mode) = GameMode::matching(&self.scope) {
            return mode.label().to_string();
        }

        let parts = self
            .scope
            .canons
            .iter()
            .map(canon_scope_label)
            .collect::<Vec<_>>();

        if parts.len() == 1 {
            return parts.into_iter().next().unwrap_or_default();
        }

        if parts.len() <= 3 && parts.iter().all(|part| part.len() <= 24) {
            return parts.join(" + ");
        }

        let selected_book_count = self
            .scope
            .canons
            .iter()
            .filter_map(|scope| match &scope.books {
                BookScope::All => None,
                BookScope::Selected(books) => Some(books.len()),
            })
            .sum::<usize>();
        let all_canon_count = self
            .scope
            .canons
            .iter()
            .filter(|scope| scope.books == BookScope::All)
            .count();

        if selected_book_count > 0 && all_canon_count > 0 {
            format!("Custom: {selected_book_count} books + {all_canon_count} full canons")
        } else if selected_book_count > 0 {
            format!(
                "Custom: {selected_book_count} books across {} canons",
                self.scope.canons.len()
            )
        } else {
            format!("Custom: {} canons", self.scope.canons.len())
        }
    }
}

fn canon_scope_label(scope: &CanonScope) -> String {
    match &scope.books {
        BookScope::All => scope.canon.label().to_string(),
        BookScope::Selected(books) if books.is_empty() => format!("No {}", scope.canon.label()),
        BookScope::Selected(books) if books.len() == 1 => format!("{} only", books[0]),
        BookScope::Selected(books) if books.len() <= 3 => books.join(" + "),
        BookScope::Selected(books) => format!("{} {} books", books.len(), scope.canon.label()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scriptures::Canon;

    fn settings(canons: Vec<CanonScope>) -> GameSettings {
        GameSettings {
            scope: GameScope { canons },
            ..GameSettings::default()
        }
    }

    #[test]
    fn labels_presets_and_custom_scopes() {
        assert_eq!(GameSettings::default().selection_label(), "Book of Mormon");
        assert_eq!(settings(Vec::new()).selection_label(), "No scope selected");
        assert_eq!(
            settings(vec![CanonScope {
                canon: Canon::BookOfMormon,
                books: BookScope::Selected(vec!["Alma".to_string()]),
            }])
            .selection_label(),
            "Alma only"
        );
    }
}
