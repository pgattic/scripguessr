mod layers;

pub use layers::LAYERS;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BookChronology {
    pub dates: &'static str,
    pub note: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub struct AtlasEra {
    pub starts_at: &'static str,
    pub name: &'static str,
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub const ERAS: &[AtlasEra] = &[
    AtlasEra {
        starts_at: "1 Nephi",
        name: "Lehi's departure and the small plates",
    },
    AtlasEra {
        starts_at: "Mosiah",
        name: "The peoples gather in Zarahemla",
    },
    AtlasEra {
        starts_at: "Alma",
        name: "The reign of the judges",
    },
    AtlasEra {
        starts_at: "3 Nephi",
        name: "The coming and ministry of Christ",
    },
    AtlasEra {
        starts_at: "4 Nephi",
        name: "Generations of peace",
    },
    AtlasEra {
        starts_at: "Mormon",
        name: "The final Nephite generations",
    },
    AtlasEra {
        starts_at: "Ether",
        name: "The Jaredite record",
    },
];

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn era_starting_at(book: &str) -> Option<AtlasEra> {
    ERAS.iter().copied().find(|era| era.starts_at == book)
}

pub fn book_chronology(book: &str) -> Option<BookChronology> {
    let (dates, note) = match book {
        "1 Nephi" => ("about 600-570 BC", "Jerusalem to the promised land"),
        "2 Nephi" => (
            "about 588-545 BC",
            "Lehi's final teachings and Nephi's record",
        ),
        "Jacob" => (
            "about 544-421 BC",
            "Jacob and Enos receive the small plates",
        ),
        "Enos" => ("about 420 BC", "Enos's ministry"),
        "Jarom" => ("about 399-361 BC", "Nephite preservation and conflict"),
        "Omni" => ("about 323-130 BC", "Several keepers of the small plates"),
        "Words of Mormon" => ("about AD 385", "Mormon's editorial bridge"),
        "Mosiah" => ("about 130-91 BC", "Kingship ends in Zarahemla"),
        "Alma" => (
            "about 91-52 BC",
            "The first thirty-nine years of the judges",
        ),
        "Helaman" => ("about 52-1 BC", "The later reign of the judges"),
        "3 Nephi" => ("AD 1-35", "Signs, upheaval, and Christ's ministry"),
        "4 Nephi" => ("about AD 36-321", "Peace, prosperity, and division"),
        "Mormon" => ("about AD 322-385", "The final wars and Mormon's record"),
        "Ether" => (
            "ancient; before 600 BC",
            "Jaredite history abridged by Moroni",
        ),
        "Moroni" => ("about AD 400-421", "Moroni's final additions"),
        _ => return None,
    };
    Some(BookChronology { dates, note })
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn layer_ids_are_unique() {
        let mut ids = LAYERS.iter().map(|layer| layer.id).collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), LAYERS.len());
    }

    #[test]
    fn every_book_has_chronology() {
        let books = [
            "1 Nephi",
            "2 Nephi",
            "Jacob",
            "Enos",
            "Jarom",
            "Omni",
            "Words of Mormon",
            "Mosiah",
            "Alma",
            "Helaman",
            "3 Nephi",
            "4 Nephi",
            "Mormon",
            "Ether",
            "Moroni",
        ];
        assert!(
            books
                .into_iter()
                .all(|book| book_chronology(book).is_some())
        );
    }
}
