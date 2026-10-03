use serde::{Deserialize, Serialize};

use super::{BookScope, CanonScope, GameScope};

#[derive(Clone, Copy, Debug, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Canon {
    BookOfMormon,
    DoctrineAndCovenants,
    PearlOfGreatPrice,
    OldTestament,
    NewTestament,
}

impl Canon {
    pub const ALL: [Self; 5] = [
        Self::OldTestament,
        Self::NewTestament,
        Self::BookOfMormon,
        Self::DoctrineAndCovenants,
        Self::PearlOfGreatPrice,
    ];

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn label(self) -> &'static str {
        match self {
            Self::BookOfMormon => "Book of Mormon",
            Self::DoctrineAndCovenants => "Doctrine and Covenants",
            Self::PearlOfGreatPrice => "Pearl of Great Price",
            Self::OldTestament => "Old Testament",
            Self::NewTestament => "New Testament",
        }
    }

    pub fn position(self) -> usize {
        Self::ALL
            .iter()
            .position(|canon| *canon == self)
            .expect("Canon::ALL lists every canon")
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum GameMode {
    BookOfMormon,
    Bible,
    Restoration,
    AllStandardWorks,
}

impl GameMode {
    pub const ALL: [Self; 4] = [
        Self::BookOfMormon,
        Self::Bible,
        Self::Restoration,
        Self::AllStandardWorks,
    ];

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn label(self) -> &'static str {
        match self {
            Self::BookOfMormon => "Book of Mormon",
            Self::Bible => "Bible",
            Self::Restoration => "Restoration",
            Self::AllStandardWorks => "All Standard Works",
        }
    }

    /// Stable identifier stored in `games.leaderboard_preset`.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub fn key(self) -> &'static str {
        match self {
            Self::BookOfMormon => "book_of_mormon",
            Self::Bible => "bible",
            Self::Restoration => "restoration",
            Self::AllStandardWorks => "all_standard_works",
        }
    }

    pub fn canons(self) -> &'static [Canon] {
        match self {
            Self::BookOfMormon => &[Canon::BookOfMormon],
            Self::Bible => &[Canon::OldTestament, Canon::NewTestament],
            Self::Restoration => &[
                Canon::BookOfMormon,
                Canon::DoctrineAndCovenants,
                Canon::PearlOfGreatPrice,
            ],
            Self::AllStandardWorks => &Canon::ALL,
        }
    }

    pub fn scope(self) -> GameScope {
        GameScope {
            canons: self
                .canons()
                .iter()
                .map(|&canon| CanonScope {
                    canon,
                    books: BookScope::All,
                })
                .collect(),
        }
    }

    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub fn matching(scope: &GameScope) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.scope() == *scope)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Difficulty {
    Easy,
    Normal,
    Hard,
}

impl Difficulty {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub const ALL: [Self; 3] = [Self::Easy, Self::Normal, Self::Hard];

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn label(self) -> &'static str {
        match self {
            Self::Easy => "Easy",
            Self::Normal => "Normal",
            Self::Hard => "Hard",
        }
    }
}
