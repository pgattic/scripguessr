#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtlasCategory {
    Person,
    Narrative,
    Event,
}

impl AtlasCategory {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn label(self) -> &'static str {
        match self {
            Self::Person => "People",
            Self::Narrative => "Narratives",
            Self::Event => "Events",
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

impl AtlasLayer {
    pub fn span_for(self, book: &str, chapter: u16) -> Option<AtlasSpan> {
        self.spans
            .iter()
            .copied()
            .find(|span| span.includes(book, chapter))
    }
}

const LEHI_JOURNEY: &[AtlasSpan] = &[AtlasSpan {
    book: "1 Nephi",
    start: 1,
    end: 18,
    note: "Lehi's family leaves Jerusalem, crosses the wilderness, builds the ship, and reaches the promised land (1 Nephi 1-18).",
}];

const KING_BENJAMIN: &[AtlasSpan] = &[AtlasSpan {
    book: "Mosiah",
    start: 1,
    end: 6,
    note: "King Benjamin transfers the kingdom and teaches his people at the temple (Mosiah 1-6).",
}];

const ABINADI: &[AtlasSpan] = &[AtlasSpan {
    book: "Mosiah",
    start: 11,
    end: 17,
    note: "Abinadi confronts King Noah, teaches of Christ, and seals his testimony with his life (Mosiah 11-17).",
}];

const ALMA_YOUNGER: &[AtlasSpan] = &[
    AtlasSpan {
        book: "Mosiah",
        start: 27,
        end: 29,
        note: "Alma the Younger is converted and begins his ministry as the reign of the judges begins (Mosiah 27-29).",
    },
    AtlasSpan {
        book: "Alma",
        start: 1,
        end: 45,
        note: "Alma serves as chief judge and high priest, leads reforming missions, and counsels his sons (Alma 1-45).",
    },
];

const SONS_OF_MOSIAH: &[AtlasSpan] = &[
    AtlasSpan {
        book: "Mosiah",
        start: 27,
        end: 28,
        note: "The sons of Mosiah are converted and depart to teach the Lamanites (Mosiah 27-28).",
    },
    AtlasSpan {
        book: "Alma",
        start: 17,
        end: 27,
        note: "Ammon and his brothers teach among the Lamanites and gather the people of Anti-Nephi-Lehi (Alma 17-27).",
    },
];

const CAPTAIN_MORONI: &[AtlasSpan] = &[AtlasSpan {
    book: "Alma",
    start: 43,
    end: 62,
    note: "Captain Moroni defends the Nephites through the major wars of the reign of the judges (Alma 43-62).",
}];

const SAMUEL: &[AtlasSpan] = &[AtlasSpan {
    book: "Helaman",
    start: 13,
    end: 16,
    note: "Samuel the Lamanite prophesies from the wall and gives signs of Christ's birth and death (Helaman 13-16).",
}];

const CHRIST_MINISTRY: &[AtlasSpan] = &[AtlasSpan {
    book: "3 Nephi",
    start: 8,
    end: 28,
    note: "Destruction marks Christ's death, and the risen Savior ministers among the people at Bountiful (3 Nephi 8-28).",
}];

const JAREDITES: &[AtlasSpan] = &[AtlasSpan {
    book: "Ether",
    start: 1,
    end: 15,
    note: "Moroni's abridgment follows the Jaredite journey, kingdoms, prophets, and final destruction (Ether 1-15).",
}];

const NEPHITE_END: &[AtlasSpan] = &[
    AtlasSpan {
        book: "Mormon",
        start: 1,
        end: 7,
        note: "Mormon leads and records the final generations of Nephite civilization through Cumorah (Mormon 1-7).",
    },
    AtlasSpan {
        book: "Moroni",
        start: 9,
        end: 9,
        note: "Mormon's final letter describes the collapse surrounding him (Moroni 9).",
    },
];

pub const LAYERS: &[AtlasLayer] = &[
    AtlasLayer {
        id: "lehi-journey",
        name: "Lehi's journey",
        category: AtlasCategory::Narrative,
        tone: "green",
        summary: "From Jerusalem through the wilderness and across the sea.",
        spans: LEHI_JOURNEY,
    },
    AtlasLayer {
        id: "king-benjamin",
        name: "King Benjamin",
        category: AtlasCategory::Person,
        tone: "gold",
        summary: "The succession, temple address, covenant, and census.",
        spans: KING_BENJAMIN,
    },
    AtlasLayer {
        id: "abinadi",
        name: "Abinadi",
        category: AtlasCategory::Person,
        tone: "red",
        summary: "Abinadi's confrontation with Noah and testimony of Christ.",
        spans: ABINADI,
    },
    AtlasLayer {
        id: "alma-younger",
        name: "Alma the Younger",
        category: AtlasCategory::Person,
        tone: "teal",
        summary: "Conversion, judgment seat, reforming ministry, and counsel.",
        spans: ALMA_YOUNGER,
    },
    AtlasLayer {
        id: "sons-mosiah",
        name: "Sons of Mosiah",
        category: AtlasCategory::Narrative,
        tone: "blue",
        summary: "Conversion and the long mission among the Lamanites.",
        spans: SONS_OF_MOSIAH,
    },
    AtlasLayer {
        id: "captain-moroni",
        name: "Captain Moroni",
        category: AtlasCategory::Person,
        tone: "orange",
        summary: "The title of liberty and the Nephite wars.",
        spans: CAPTAIN_MORONI,
    },
    AtlasLayer {
        id: "samuel",
        name: "Samuel the Lamanite",
        category: AtlasCategory::Person,
        tone: "violet",
        summary: "Prophecies and signs delivered from the wall.",
        spans: SAMUEL,
    },
    AtlasLayer {
        id: "christ-ministry",
        name: "Christ in Bountiful",
        category: AtlasCategory::Event,
        tone: "yellow",
        summary: "The signs of Christ's death and his ministry in the Americas.",
        spans: CHRIST_MINISTRY,
    },
    AtlasLayer {
        id: "jaredite-record",
        name: "Jaredite record",
        category: AtlasCategory::Narrative,
        tone: "cyan",
        summary: "The complete rise and fall of the Jaredite civilization.",
        spans: JAREDITES,
    },
    AtlasLayer {
        id: "nephite-end",
        name: "End of the Nephites",
        category: AtlasCategory::Event,
        tone: "rose",
        summary: "Mormon's account of the final wars and collapse.",
        spans: NEPHITE_END,
    },
];

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
}
