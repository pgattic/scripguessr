use std::collections::BTreeSet;

use super::{Difficulty, Verse};

/// Indices of verses worth guessing at `difficulty`. Falls back to every verse
/// so a canon never has an empty pool.
pub(super) fn playable_indices(verses: &[Verse], difficulty: Difficulty) -> Vec<usize> {
    let playable = verses
        .iter()
        .enumerate()
        .filter(|(_, verse)| is_playable_verse(&verse.text, difficulty))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();

    if playable.is_empty() {
        (0..verses.len()).collect()
    } else {
        playable
    }
}

fn is_playable_verse(text: &str, difficulty: Difficulty) -> bool {
    let normalized_words = normalized_words(text);
    let word_count = normalized_words.len();
    let minimum_words = match difficulty {
        Difficulty::Easy => 24,
        Difficulty::Normal => 16,
        Difficulty::Hard => 8,
    };

    if word_count < minimum_words {
        return false;
    }

    let distinct_words = normalized_words.iter().collect::<BTreeSet<_>>().len();

    let minimum_distinct_words = match difficulty {
        Difficulty::Easy => 16,
        Difficulty::Normal => 10,
        Difficulty::Hard => 6,
    };

    if distinct_words < minimum_distinct_words {
        return false;
    }

    let lower = text.to_lowercase();
    let filler_hits = [
        "and it came to pass",
        "now behold",
        "and now",
        "yea",
        "therefore",
    ]
    .iter()
    .filter(|phrase| lower.contains(*phrase))
    .count();

    let distinct_ratio = distinct_words as f32 / word_count as f32;
    match difficulty {
        Difficulty::Easy => filler_hits <= 1 || distinct_ratio >= 0.8,
        Difficulty::Normal => filler_hits <= 2 || distinct_ratio >= 0.72,
        Difficulty::Hard => filler_hits <= 2 || distinct_ratio >= 0.62,
    }
}

fn normalized_words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|word| {
            word.chars()
                .filter(|character| character.is_alphanumeric() || *character == '\'')
                .collect::<String>()
                .to_lowercase()
        })
        .filter(|word| !word.is_empty())
        .collect()
}
