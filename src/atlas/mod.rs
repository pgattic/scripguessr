mod chronology;
mod layers;

#[cfg_attr(not(target_arch = "wasm32"), allow(unused_imports))]
pub use chronology::{book_chronology, chronology_title};
pub use layers::LAYERS;

use crate::scriptures::{BookInfo, Canon};
use crate::study_sets::{PromptPolicy, StudyGuessScope, StudyPassage, StudySet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtlasCategory {
    Person,
    Narrative,
    Event,
    Teaching,
}

impl AtlasCategory {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub const ALL: [Self; 4] = [Self::Person, Self::Narrative, Self::Event, Self::Teaching];

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn label(self) -> &'static str {
        match self {
            Self::Person => "People",
            Self::Narrative => "Narratives",
            Self::Event => "Events",
            Self::Teaching => "Teachings",
        }
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub const fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasSpan {
    pub book: &'static str,
    pub start: u16,
    pub end: u16,
    pub note: &'static str,
}

impl AtlasSpan {
    pub fn includes(self, book: &str, chapter: u16) -> bool {
        self.book == book && (self.start..=self.end).contains(&chapter)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasLayer {
    pub id: &'static str,
    pub name: &'static str,
    pub category: AtlasCategory,
    pub tone: &'static str,
    pub summary: &'static str,
    pub spans: &'static [AtlasSpan],
}

impl AtlasLayer {
    pub fn span_for(self, book: &str, chapter: u16) -> Option<AtlasSpan> {
        self.spans
            .iter()
            .copied()
            .find(|span| span.includes(book, chapter))
    }
}

pub fn layer(id: &str) -> Option<AtlasLayer> {
    LAYERS.iter().copied().find(|layer| layer.id == id)
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn layers_covering(selected: &[&str], book: &str, chapter: u16) -> Vec<AtlasLayer> {
    selected
        .iter()
        .filter_map(|id| layer(id))
        .filter(|layer| layer.span_for(book, chapter).is_some())
        .collect()
}

/// A study set of every whole chapter covered by the selected layers.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn practice_set(selected: &[&str], books: &[BookInfo]) -> Option<StudySet> {
    let layers = selected
        .iter()
        .filter_map(|id| layer(id))
        .collect::<Vec<_>>();
    let passages = books
        .iter()
        .flat_map(|book| book.chapters.iter().map(move |&chapter| (book, chapter)))
        .filter(|(book, chapter)| {
            layers
                .iter()
                .any(|layer| layer.span_for(&book.name, *chapter).is_some())
        })
        .filter_map(|(book, chapter)| {
            Some(StudyPassage {
                canon: Canon::BookOfMormon,
                book: book.name.clone(),
                chapter,
                verses: (1..=book.verse_count(chapter)?).collect(),
            })
        })
        .collect::<Vec<_>>();
    if passages.is_empty() {
        return None;
    }
    let name = match layers.as_slice() {
        [layer] => layer.name.to_string(),
        _ => "Atlas selection".to_string(),
    };
    Some(StudySet {
        id: "atlas-selection".to_string(),
        name,
        passages,
        guess_scope: StudyGuessScope::FullCanons,
        prompt_policy: PromptPolicy::SingleVerse,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_span_resolves_from_scripture_data() {
        let library = crate::scriptures::ScriptureLibrary::standard_works().unwrap();
        let books = &library.scriptures(Canon::BookOfMormon).unwrap().books;

        for span in LAYERS.iter().flat_map(|layer| layer.spans) {
            let book = books
                .iter()
                .find(|book| book.name == span.book)
                .unwrap_or_else(|| panic!("atlas book not found: {}", span.book));
            assert!(
                (span.start..=span.end).all(|chapter| book.chapters.contains(&chapter)),
                "atlas span not found: {} {}-{}",
                span.book,
                span.start,
                span.end
            );
        }
    }

    #[test]
    fn practice_sets_cover_whole_chapters_of_selected_layers() {
        let library = crate::scriptures::ScriptureLibrary::standard_works().unwrap();
        let books = &library.scriptures(Canon::BookOfMormon).unwrap().books;

        let set = practice_set(&["abinadi"], books).unwrap();

        assert_eq!(set.name, "Abinadi");
        assert_eq!(set.passages.len(), 7);
        assert!(set.passages.iter().all(|passage| passage.book == "Mosiah"));
        assert!(practice_set(&[], books).is_none());
        assert_eq!(
            layers_covering(&["abinadi", "king-benjamin"], "Mosiah", 12).len(),
            1
        );
    }

    #[test]
    fn overlapping_layers_are_independently_discoverable() {
        assert!(
            layer("alma-younger")
                .unwrap()
                .span_for("Alma", 20)
                .is_some()
        );
        assert!(layer("sons-mosiah").unwrap().span_for("Alma", 20).is_some());
        assert!(
            layer("captain-moroni")
                .unwrap()
                .span_for("Alma", 20)
                .is_none()
        );
    }

    #[test]
    fn every_span_has_valid_bounds() {
        assert!(
            LAYERS
                .iter()
                .flat_map(|layer| layer.spans)
                .all(|span| { !span.book.is_empty() && span.start > 0 && span.start <= span.end })
        );
    }

    #[test]
    fn every_category_has_a_useful_catalog() {
        for category in AtlasCategory::ALL {
            assert!(
                LAYERS
                    .iter()
                    .filter(|layer| layer.category == category)
                    .count()
                    >= 8,
                "{} needs more layers",
                category.label()
            );
        }
    }

    #[test]
    fn layer_ids_are_unique() {
        let mut ids = LAYERS.iter().map(|layer| layer.id).collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), LAYERS.len());
    }
}
