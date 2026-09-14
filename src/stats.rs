use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::scriptures::{Difficulty, Reference};

#[cfg(target_arch = "wasm32")]
const STORAGE_KEY: &str = "scripguessr.stats.v1";

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Stats {
    pub games_played: u32,
    pub rounds_played: u32,
    pub total_score: u32,
    pub total_possible_score: u32,
    pub best_score: Option<ScoreMark>,
    pub best_by_difficulty: BTreeMap<Difficulty, ScoreMark>,
    pub books: BTreeMap<String, BookStats>,
    #[serde(default)]
    pub review_items: Vec<ReviewItem>,
}

impl Stats {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn load() -> Self {
        load_stats().unwrap_or_default()
    }

    pub fn save(&self) {
        save_stats(self);
    }

    pub fn record_game(&mut self, game: FinishedGame) -> bool {
        self.games_played += 1;
        self.rounds_played += game.rounds.len() as u32;
        self.total_score += game.score;
        self.total_possible_score += game.possible_score;

        let mark = ScoreMark {
            score: game.score,
            possible_score: game.possible_score,
            difficulty: game.difficulty,
            round_count: game.rounds.len(),
        };

        let new_overall_best = self
            .best_score
            .map(|best| mark.score_percent() > best.score_percent())
            .unwrap_or(true);
        if new_overall_best {
            self.best_score = Some(mark);
        }

        let difficulty_best = self
            .best_by_difficulty
            .entry(game.difficulty)
            .or_insert(mark);
        if mark.score_percent() > difficulty_best.score_percent() {
            *difficulty_best = mark;
        }

        for round in game.rounds {
            self.books
                .entry(round.answer_book)
                .or_default()
                .record_round(round.score, round.possible_score);
        }

        self.save();
        new_overall_best
    }

    pub fn average_percent(&self) -> Option<u32> {
        percent(self.total_score, self.total_possible_score)
    }

    pub fn weakest_books(&self, limit: usize) -> Vec<(&String, &BookStats)> {
        let mut books = self
            .books
            .iter()
            .filter(|(_book, stats)| stats.rounds_played > 0)
            .collect::<Vec<_>>();
        books.sort_by_key(|(_book, stats)| stats.average_percent().unwrap_or(u32::MAX));
        books.into_iter().take(limit).collect()
    }

    pub fn is_marked_for_review(&self, reference: &Reference) -> bool {
        self.review_items
            .iter()
            .any(|item| item.reference == *reference)
    }

    pub fn toggle_review_item(&mut self, item: ReviewItem) -> bool {
        if let Some(index) = self
            .review_items
            .iter()
            .position(|candidate| candidate.reference == item.reference)
        {
            self.review_items.remove(index);
            self.save();
            false
        } else {
            self.review_items.push(item);
            self.save();
            true
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ScoreMark {
    pub score: u32,
    pub possible_score: u32,
    pub difficulty: Difficulty,
    pub round_count: usize,
}

impl ScoreMark {
    pub fn score_percent(self) -> u32 {
        percent(self.score, self.possible_score).unwrap_or(0)
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct BookStats {
    pub rounds_played: u32,
    pub total_score: u32,
    pub total_possible_score: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ReviewItem {
    pub reference: Reference,
    pub text: String,
    pub score: u32,
}

impl BookStats {
    fn record_round(&mut self, score: u32, possible_score: u32) {
        self.rounds_played += 1;
        self.total_score += score;
        self.total_possible_score += possible_score;
    }

    pub fn average_percent(&self) -> Option<u32> {
        percent(self.total_score, self.total_possible_score)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FinishedGame {
    pub difficulty: Difficulty,
    pub score: u32,
    pub possible_score: u32,
    pub rounds: Vec<FinishedRound>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FinishedRound {
    pub answer_book: String,
    pub score: u32,
    pub possible_score: u32,
}

fn percent(score: u32, possible_score: u32) -> Option<u32> {
    if possible_score == 0 {
        None
    } else {
        Some(((score as f64 / possible_score as f64) * 100.0).round() as u32)
    }
}

#[cfg(target_arch = "wasm32")]
fn load_stats() -> Option<Stats> {
    let storage = web_sys::window()?.local_storage().ok()??;
    let json = storage.get_item(STORAGE_KEY).ok()??;
    serde_json::from_str(&json).ok()
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn load_stats() -> Option<Stats> {
    None
}

#[cfg(target_arch = "wasm32")]
fn save_stats(stats: &Stats) {
    let Some(storage) = web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    else {
        return;
    };
    let Ok(json) = serde_json::to_string(stats) else {
        return;
    };
    let _ = storage.set_item(STORAGE_KEY, &json);
}

#[cfg(not(target_arch = "wasm32"))]
fn save_stats(_stats: &Stats) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn round(book: &str, score: u32) -> FinishedRound {
        FinishedRound {
            answer_book: book.to_string(),
            score,
            possible_score: 1000,
        }
    }

    #[test]
    fn records_totals_and_best_score() {
        let mut stats = Stats::default();

        let new_best = stats.record_game(FinishedGame {
            difficulty: Difficulty::Normal,
            score: 1600,
            possible_score: 2000,
            rounds: vec![round("Alma", 900), round("Mosiah", 700)],
        });

        assert!(new_best);
        assert_eq!(stats.games_played, 1);
        assert_eq!(stats.rounds_played, 2);
        assert_eq!(stats.average_percent(), Some(80));
        assert_eq!(stats.best_score.unwrap().score_percent(), 80);
        assert_eq!(
            stats
                .best_by_difficulty
                .get(&Difficulty::Normal)
                .unwrap()
                .score,
            1600
        );
    }

    #[test]
    fn lower_score_is_not_a_new_best() {
        let mut stats = Stats::default();
        stats.record_game(FinishedGame {
            difficulty: Difficulty::Normal,
            score: 900,
            possible_score: 1000,
            rounds: vec![round("Alma", 900)],
        });

        let new_best = stats.record_game(FinishedGame {
            difficulty: Difficulty::Hard,
            score: 700,
            possible_score: 1000,
            rounds: vec![round("Alma", 700)],
        });

        assert!(!new_best);
        assert_eq!(stats.best_score.unwrap().score, 900);
    }

    #[test]
    fn weakest_books_are_sorted_by_average_percent() {
        let mut stats = Stats::default();
        stats.record_game(FinishedGame {
            difficulty: Difficulty::Normal,
            score: 1200,
            possible_score: 3000,
            rounds: vec![
                round("Alma", 800),
                round("Mosiah", 300),
                round("Jacob", 100),
            ],
        });

        let weakest = stats.weakest_books(2);

        assert_eq!(weakest[0].0, "Jacob");
        assert_eq!(weakest[1].0, "Mosiah");
    }

    #[test]
    fn review_items_toggle_by_reference() {
        let mut stats = Stats::default();
        let item = ReviewItem {
            reference: Reference {
                canon: crate::scriptures::Canon::BookOfMormon,
                book: "Alma".to_string(),
                chapter: 32,
                verse: 21,
            },
            text: "And now as I said concerning faith".to_string(),
            score: 612,
        };

        assert!(stats.toggle_review_item(item.clone()));
        assert!(stats.is_marked_for_review(&item.reference));
        assert_eq!(stats.review_items.len(), 1);

        assert!(!stats.toggle_review_item(item.clone()));
        assert!(!stats.is_marked_for_review(&item.reference));
        assert!(stats.review_items.is_empty());
    }
}
