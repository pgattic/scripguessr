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

/// Hover text for a book's date range.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn chronology_title(book: &str) -> Option<String> {
    let chronology = book_chronology(book)?;
    Some(match era_starting_at(book) {
        Some(era) => format!("{}. {}. Dates are approximate.", chronology.note, era.name),
        None => format!("{}. Dates are approximate.", chronology.note),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
