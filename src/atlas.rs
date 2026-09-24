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

const NEPHI_MINISTRY: &[AtlasSpan] = &[
    AtlasSpan {
        book: "1 Nephi",
        start: 1,
        end: 22,
        note: "Nephi records his family's journey, visions, conflicts, and arrival in the promised land (1 Nephi 1-22).",
    },
    AtlasSpan {
        book: "2 Nephi",
        start: 1,
        end: 33,
        note: "Nephi preserves Lehi's final teachings, establishes a new community, and closes his record with testimony of Christ (2 Nephi 1-33).",
    },
];

const JACOB_MINISTRY: &[AtlasSpan] = &[
    AtlasSpan {
        book: "2 Nephi",
        start: 6,
        end: 10,
        note: "Jacob teaches from Isaiah and testifies of the Atonement and the gathering of Israel (2 Nephi 6-10).",
    },
    AtlasSpan {
        book: "Jacob",
        start: 1,
        end: 7,
        note: "Jacob confronts pride and immorality, teaches the allegory of the olive tree, and faces Sherem (Jacob 1-7).",
    },
];

const PEOPLE_OF_ZENIFF: &[AtlasSpan] = &[AtlasSpan {
    book: "Mosiah",
    start: 7,
    end: 24,
    note: "The record of Zeniff's colony follows Limhi's people and Alma's people through bondage and deliverance (Mosiah 7-24).",
}];

const PEOPLE_OF_AMMON: &[AtlasSpan] = &[
    AtlasSpan {
        book: "Alma",
        start: 23,
        end: 27,
        note: "Converted Lamanites covenant against bloodshed and are established in Jershon as the people of Ammon (Alma 23-27).",
    },
    AtlasSpan {
        book: "Alma",
        start: 53,
        end: 58,
        note: "Their sons enter the war under Helaman and become known for faith and courage (Alma 53-58).",
    },
];

const STRIPLING_WARRIORS: &[AtlasSpan] = &[
    AtlasSpan {
        book: "Alma",
        start: 53,
        end: 53,
        note: "Two thousand sons of the people of Ammon covenant to defend the Nephites (Alma 53).",
    },
    AtlasSpan {
        book: "Alma",
        start: 56,
        end: 58,
        note: "Helaman recounts the young warriors' battles, faith, wounds, and preservation (Alma 56-58).",
    },
];

const NEPHI_AND_LEHI: &[AtlasSpan] = &[AtlasSpan {
    book: "Helaman",
    start: 3,
    end: 12,
    note: "Nephi and Lehi preach amid war and secret combinations; Nephi later receives the sealing power (Helaman 3-12).",
}];

const GADIANTON_ROBBERS: &[AtlasSpan] = &[
    AtlasSpan {
        book: "Helaman",
        start: 1,
        end: 11,
        note: "Secret combinations spread through Nephite government and society (Helaman 1-11).",
    },
    AtlasSpan {
        book: "3 Nephi",
        start: 1,
        end: 4,
        note: "The gathered Nephites and Lamanites defeat the Gadianton armies (3 Nephi 1-4).",
    },
    AtlasSpan {
        book: "3 Nephi",
        start: 6,
        end: 7,
        note: "Prosperity, division, and secret combinations help collapse the government (3 Nephi 6-7).",
    },
];

const ISAIAH_PASSAGES: &[AtlasSpan] = &[
    AtlasSpan {
        book: "1 Nephi",
        start: 20,
        end: 21,
        note: "Nephi includes Isaiah's words about covenant deliverance and the gathering of Israel (1 Nephi 20-21).",
    },
    AtlasSpan {
        book: "2 Nephi",
        start: 6,
        end: 8,
        note: "Jacob uses Isaiah to teach of the Messiah and the gathering of Israel (2 Nephi 6-8).",
    },
    AtlasSpan {
        book: "2 Nephi",
        start: 12,
        end: 24,
        note: "Nephi records an extended block of Isaiah and then explains its latter-day fulfillment (2 Nephi 12-24).",
    },
];

const DOCTRINE_OF_CHRIST: &[AtlasSpan] = &[AtlasSpan {
    book: "2 Nephi",
    start: 31,
    end: 33,
    note: "Nephi teaches faith in Christ, repentance, baptism, the Holy Ghost, and enduring to the end (2 Nephi 31-33).",
}];

const ALMA_TEACHINGS: &[AtlasSpan] = &[
    AtlasSpan {
        book: "Alma",
        start: 5,
        end: 7,
        note: "Alma calls the people of Zarahemla and Gideon to spiritual rebirth and faith in Christ (Alma 5-7).",
    },
    AtlasSpan {
        book: "Alma",
        start: 12,
        end: 13,
        note: "Alma and Amulek teach Zeezrom about redemption, resurrection, judgment, and priesthood (Alma 12-13).",
    },
    AtlasSpan {
        book: "Alma",
        start: 32,
        end: 34,
        note: "Alma and Amulek teach the Zoramites about faith, the word, prayer, and the Atonement (Alma 32-34).",
    },
    AtlasSpan {
        book: "Alma",
        start: 36,
        end: 42,
        note: "Alma recounts his conversion and counsels his sons about scripture, agency, resurrection, and justice (Alma 36-42).",
    },
];

const SIGNS_OF_CHRIST: &[AtlasSpan] = &[
    AtlasSpan {
        book: "3 Nephi",
        start: 1,
        end: 1,
        note: "The night without darkness fulfills Samuel's sign of Christ's birth (3 Nephi 1).",
    },
    AtlasSpan {
        book: "3 Nephi",
        start: 8,
        end: 10,
        note: "Destruction and darkness mark Christ's death, followed by his voice inviting the survivors to return (3 Nephi 8-10).",
    },
];

const FOURTH_NEPHI_PEACE: &[AtlasSpan] = &[AtlasSpan {
    book: "4 Nephi",
    start: 1,
    end: 1,
    note: "Christ's disciples establish generations of unity and peace before pride and division return (4 Nephi 1).",
}];

const ENOS: &[AtlasSpan] = &[AtlasSpan {
    book: "Enos",
    start: 1,
    end: 1,
    note: "Enos prays for forgiveness, then for the Nephites and Lamanites, and receives covenant assurances (Enos 1).",
}];

const ALMA_ELDER: &[AtlasSpan] = &[
    AtlasSpan {
        book: "Mosiah",
        start: 17,
        end: 18,
        note: "Alma believes Abinadi, records his words, and organizes the church at the Waters of Mormon (Mosiah 17-18).",
    },
    AtlasSpan {
        book: "Mosiah",
        start: 23,
        end: 26,
        note: "Alma's people are delivered from bondage, join Mosiah's people, and establish churches throughout Zarahemla (Mosiah 23-26).",
    },
];

const AMULEK: &[AtlasSpan] = &[
    AtlasSpan {
        book: "Alma",
        start: 8,
        end: 16,
        note: "Amulek joins Alma in Ammonihah, testifies of his conversion, and teaches through persecution and deliverance (Alma 8-16).",
    },
    AtlasSpan {
        book: "Alma",
        start: 31,
        end: 35,
        note: "Amulek accompanies the mission to the Zoramites and teaches them about prayer and Christ's Atonement (Alma 31-35).",
    },
];

const HELAMAN: &[AtlasSpan] = &[
    AtlasSpan {
        book: "Alma",
        start: 36,
        end: 37,
        note: "Alma recounts his conversion to Helaman and entrusts him with the sacred records (Alma 36-37).",
    },
    AtlasSpan {
        book: "Alma",
        start: 45,
        end: 45,
        note: "Helaman succeeds Alma in the ministry and preserves his father's prophecy (Alma 45).",
    },
    AtlasSpan {
        book: "Alma",
        start: 53,
        end: 58,
        note: "Helaman leads the sons of the people of Ammon and reports their campaigns to Moroni (Alma 53-58).",
    },
];

const MORMON_MINISTRY: &[AtlasSpan] = &[AtlasSpan {
    book: "Mormon",
    start: 1,
    end: 7,
    note: "Mormon becomes the record keeper and military leader, preaches repentance, and witnesses the destruction at Cumorah (Mormon 1-7).",
}];

const MORONI_WITNESS: &[AtlasSpan] = &[
    AtlasSpan {
        book: "Mormon",
        start: 8,
        end: 9,
        note: "Moroni completes his father's record and addresses future readers who will receive it (Mormon 8-9).",
    },
    AtlasSpan {
        book: "Ether",
        start: 12,
        end: 12,
        note: "Moroni pauses the Jaredite account to teach about faith, hope, weakness, and grace (Ether 12).",
    },
    AtlasSpan {
        book: "Moroni",
        start: 1,
        end: 10,
        note: "Moroni preserves ordinances, teachings, letters, and his final invitation to seek a witness from God (Moroni 1-10).",
    },
];

const BRASS_PLATES: &[AtlasSpan] = &[AtlasSpan {
    book: "1 Nephi",
    start: 3,
    end: 5,
    note: "Lehi's sons return to Jerusalem, obtain the brass plates from Laban, and bring them into the wilderness (1 Nephi 3-5).",
}];

const WATERS_OF_MORMON: &[AtlasSpan] = &[AtlasSpan {
    book: "Mosiah",
    start: 18,
    end: 18,
    note: "Alma teaches the baptismal covenant and organizes the church at the Waters of Mormon (Mosiah 18).",
}];

const AMMON_AND_LAMONI: &[AtlasSpan] = &[AtlasSpan {
    book: "Alma",
    start: 17,
    end: 20,
    note: "Ammon serves and teaches King Lamoni, whose household receives a witness of Christ (Alma 17-20).",
}];

const ZORAMITE_MISSION: &[AtlasSpan] = &[AtlasSpan {
    book: "Alma",
    start: 31,
    end: 35,
    note: "Alma and his companions encounter worship on the Rameumptom and teach the poor Zoramites faith in Christ (Alma 31-35).",
}];

const NEPHITE_WARS: &[AtlasSpan] = &[AtlasSpan {
    book: "Alma",
    start: 43,
    end: 63,
    note: "A long sequence of wars includes the title of liberty, fortified cities, internal rebellion, and campaigns on two fronts (Alma 43-63).",
}];

const OLIVE_TREE: &[AtlasSpan] = &[AtlasSpan {
    book: "Jacob",
    start: 5,
    end: 6,
    note: "Jacob records Zenos's allegory of the olive trees and invites Israel to labor with the Lord of the vineyard (Jacob 5-6).",
}];

const SERMON_AT_TEMPLE: &[AtlasSpan] = &[AtlasSpan {
    book: "3 Nephi",
    start: 12,
    end: 14,
    note: "Christ teaches the people at Bountiful the higher law and the pattern of covenant discipleship (3 Nephi 12-14).",
}];

const GATHERING_ISRAEL: &[AtlasSpan] = &[
    AtlasSpan {
        book: "3 Nephi",
        start: 15,
        end: 16,
        note: "Christ identifies the people as other sheep and explains the gathering of covenant Israel (3 Nephi 15-16).",
    },
    AtlasSpan {
        book: "3 Nephi",
        start: 20,
        end: 22,
        note: "Christ teaches the covenant, gathering, and latter-day work using Isaiah and Micah (3 Nephi 20-22).",
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
    AtlasLayer {
        id: "nephi-ministry",
        name: "Nephi's ministry",
        category: AtlasCategory::Person,
        tone: "cyan",
        summary: "Nephi's visions, leadership, teaching, and final testimony.",
        spans: NEPHI_MINISTRY,
    },
    AtlasLayer {
        id: "jacob-ministry",
        name: "Jacob's ministry",
        category: AtlasCategory::Person,
        tone: "violet",
        summary: "Jacob's sermons, stewardship, and defense of the faith.",
        spans: JACOB_MINISTRY,
    },
    AtlasLayer {
        id: "people-zeniff",
        name: "People of Zeniff",
        category: AtlasCategory::Narrative,
        tone: "orange",
        summary: "The colony in Lehi-Nephi, its bondage, and deliverance.",
        spans: PEOPLE_OF_ZENIFF,
    },
    AtlasLayer {
        id: "people-ammon",
        name: "People of Ammon",
        category: AtlasCategory::Narrative,
        tone: "green",
        summary: "A covenant of peace carried into the next generation.",
        spans: PEOPLE_OF_AMMON,
    },
    AtlasLayer {
        id: "stripling-warriors",
        name: "Stripling warriors",
        category: AtlasCategory::Narrative,
        tone: "gold",
        summary: "Helaman's young army and the faith that sustains them.",
        spans: STRIPLING_WARRIORS,
    },
    AtlasLayer {
        id: "nephi-lehi",
        name: "Nephi and Lehi",
        category: AtlasCategory::Person,
        tone: "blue",
        summary: "Missionary work, prophecy, and the sealing power.",
        spans: NEPHI_AND_LEHI,
    },
    AtlasLayer {
        id: "gadianton-robbers",
        name: "Gadianton robbers",
        category: AtlasCategory::Narrative,
        tone: "red",
        summary: "The growth of secret combinations and their consequences.",
        spans: GADIANTON_ROBBERS,
    },
    AtlasLayer {
        id: "isaiah-passages",
        name: "Isaiah passages",
        category: AtlasCategory::Teaching,
        tone: "violet",
        summary: "Isaiah on the Messiah, covenant Israel, and latter days.",
        spans: ISAIAH_PASSAGES,
    },
    AtlasLayer {
        id: "doctrine-christ",
        name: "Doctrine of Christ",
        category: AtlasCategory::Teaching,
        tone: "teal",
        summary: "The covenant path in Nephi's concluding teachings.",
        spans: DOCTRINE_OF_CHRIST,
    },
    AtlasLayer {
        id: "alma-teachings",
        name: "Alma's major teachings",
        category: AtlasCategory::Teaching,
        tone: "blue",
        summary: "Spiritual rebirth, faith, redemption, and resurrection.",
        spans: ALMA_TEACHINGS,
    },
    AtlasLayer {
        id: "signs-christ",
        name: "Signs of Christ",
        category: AtlasCategory::Event,
        tone: "yellow",
        summary: "The promised signs surrounding Christ's birth and death.",
        spans: SIGNS_OF_CHRIST,
    },
    AtlasLayer {
        id: "fourth-nephi-peace",
        name: "Fourth Nephi peace",
        category: AtlasCategory::Event,
        tone: "green",
        summary: "A Zion society followed by renewed division.",
        spans: FOURTH_NEPHI_PEACE,
    },
    AtlasLayer {
        id: "enos",
        name: "Enos",
        category: AtlasCategory::Person,
        tone: "gold",
        summary: "A prayer of repentance, intercession, and covenant assurance.",
        spans: ENOS,
    },
    AtlasLayer {
        id: "alma-elder",
        name: "Alma the Elder",
        category: AtlasCategory::Person,
        tone: "green",
        summary: "From Abinadi's court to leadership of the church.",
        spans: ALMA_ELDER,
    },
    AtlasLayer {
        id: "amulek",
        name: "Amulek",
        category: AtlasCategory::Person,
        tone: "orange",
        summary: "Conversion and ministry beside Alma in two major missions.",
        spans: AMULEK,
    },
    AtlasLayer {
        id: "helaman",
        name: "Helaman",
        category: AtlasCategory::Person,
        tone: "blue",
        summary: "Record keeper, high priest, and leader of the young warriors.",
        spans: HELAMAN,
    },
    AtlasLayer {
        id: "mormon-ministry",
        name: "Mormon",
        category: AtlasCategory::Person,
        tone: "teal",
        summary: "Prophet, historian, and witness of his people's fall.",
        spans: MORMON_MINISTRY,
    },
    AtlasLayer {
        id: "moroni-witness",
        name: "Moroni",
        category: AtlasCategory::Person,
        tone: "violet",
        summary: "The final record keeper's witness to future readers.",
        spans: MORONI_WITNESS,
    },
    AtlasLayer {
        id: "brass-plates",
        name: "Obtaining the brass plates",
        category: AtlasCategory::Narrative,
        tone: "cyan",
        summary: "The return to Jerusalem and preservation of scripture.",
        spans: BRASS_PLATES,
    },
    AtlasLayer {
        id: "waters-mormon",
        name: "Waters of Mormon",
        category: AtlasCategory::Event,
        tone: "green",
        summary: "A baptismal covenant and the beginning of a church community.",
        spans: WATERS_OF_MORMON,
    },
    AtlasLayer {
        id: "ammon-lamoni",
        name: "Ammon and Lamoni",
        category: AtlasCategory::Narrative,
        tone: "yellow",
        summary: "Service, conversion, and revelation in the land of Ishmael.",
        spans: AMMON_AND_LAMONI,
    },
    AtlasLayer {
        id: "zoramite-mission",
        name: "Zoramite mission",
        category: AtlasCategory::Narrative,
        tone: "rose",
        summary: "The Rameumptom and teaching faith among the poor.",
        spans: ZORAMITE_MISSION,
    },
    AtlasLayer {
        id: "nephite-wars",
        name: "Nephite wars",
        category: AtlasCategory::Event,
        tone: "red",
        summary: "The extended military struggle late in the book of Alma.",
        spans: NEPHITE_WARS,
    },
    AtlasLayer {
        id: "olive-tree",
        name: "Allegory of the olive tree",
        category: AtlasCategory::Teaching,
        tone: "green",
        summary: "Zenos's panorama of scattering, gathering, and covenant labor.",
        spans: OLIVE_TREE,
    },
    AtlasLayer {
        id: "sermon-temple",
        name: "Sermon at the temple",
        category: AtlasCategory::Teaching,
        tone: "gold",
        summary: "Christ's higher law and pattern of discipleship.",
        spans: SERMON_AT_TEMPLE,
    },
    AtlasLayer {
        id: "gathering-israel",
        name: "Gathering of Israel",
        category: AtlasCategory::Teaching,
        tone: "cyan",
        summary: "Christ explains covenant Israel and the latter-day gathering.",
        spans: GATHERING_ISRAEL,
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
