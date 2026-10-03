use std::collections::HashSet;

use rand::Rng;
use rand::seq::SliceRandom;

use crate::scriptures::{Difficulty, GameScope, ScriptureLibrary, Verse};
use crate::server::ApiError;
use crate::study_sets::{PromptPolicy, StudyPassage};

#[derive(Clone)]
pub struct RoundAnswer {
    /// The verses shown to the player.
    pub passage: StudyPassage,
    /// The study passage the shown verses were drawn from.
    pub source_passage: StudyPassage,
    pub text: String,
}

impl From<&Verse> for RoundAnswer {
    fn from(verse: &Verse) -> Self {
        let passage = StudyPassage::single(&verse.reference);
        Self {
            source_passage: passage.clone(),
            passage,
            text: verse.text.clone(),
        }
    }
}

pub fn random_rounds(
    library: &ScriptureLibrary,
    difficulty: Difficulty,
    scope: &GameScope,
    round_count: usize,
    playable_count: usize,
    rng: &mut impl Rng,
) -> Result<Vec<RoundAnswer>, ApiError> {
    if playable_count == 0 {
        return Err(no_playable_verses());
    }
    (0..round_count)
        .map(|_| {
            let index = rng.random_range(0..playable_count);
            library
                .nth_playable_verse(difficulty, scope, index)
                .map(RoundAnswer::from)
                .ok_or_else(no_playable_verses)
        })
        .collect()
}

/// Picks up to `round_count` distinct passages in random order.
pub fn study_rounds(
    library: &ScriptureLibrary,
    passages: Vec<StudyPassage>,
    round_count: usize,
    prompt_policy: PromptPolicy,
    rng: &mut impl Rng,
) -> Result<Vec<RoundAnswer>, ApiError> {
    let mut seen = HashSet::with_capacity(passages.len());
    let mut selected = passages
        .into_iter()
        .filter(|passage| seen.insert(passage.clone()))
        .collect::<Vec<_>>();
    selected.shuffle(rng);
    selected.truncate(round_count);

    let rounds = selected
        .into_iter()
        .map(|passage| {
            let shown_passage = if prompt_policy.shows_whole_passage(&passage) {
                passage.clone()
            } else {
                let index = rng.random_range(0..passage.verses.len());
                StudyPassage {
                    verses: vec![passage.verses[index]],
                    ..passage.clone()
                }
            };
            let verse = library
                .scriptures(passage.canon)
                .and_then(|scriptures| {
                    scriptures.passage(
                        &shown_passage.book,
                        shown_passage.chapter,
                        &shown_passage.verses,
                    )
                })
                .ok_or_else(|| ApiError::bad_request("Study passage was not found"))?;
            Ok(RoundAnswer {
                passage: shown_passage,
                source_passage: passage,
                text: verse.text,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;

    if rounds.is_empty() {
        return Err(ApiError::bad_request("No study passages found"));
    }
    Ok(rounds)
}

fn no_playable_verses() -> ApiError {
    ApiError::bad_request("No playable verses found")
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    use super::*;
    use crate::scriptures::GameMode;
    use crate::study_sets::built_in_study_sets;

    #[test]
    fn every_curated_study_passage_resolves_from_scripture_data() {
        let library = ScriptureLibrary::standard_works().unwrap();
        let mut rng = SmallRng::seed_from_u64(1);
        for set in built_in_study_sets() {
            let expected_count = set.passages.len();
            let rounds = study_rounds(
                &library,
                set.passages.clone(),
                expected_count,
                PromptPolicy::WholePassage,
                &mut rng,
            )
            .unwrap();
            assert_eq!(rounds.len(), expected_count, "set: {}", set.name);
            assert!(
                rounds.iter().all(|round| !round.text.is_empty()),
                "set: {}",
                set.name
            );
        }
    }

    #[test]
    fn single_verse_prompts_come_from_their_source_passage() {
        let library = ScriptureLibrary::standard_works().unwrap();
        let mut rng = SmallRng::seed_from_u64(7);
        let source = StudyPassage {
            canon: crate::scriptures::Canon::BookOfMormon,
            book: "Moroni".to_string(),
            chapter: 7,
            verses: vec![45, 46, 47, 48],
        };

        let rounds = study_rounds(
            &library,
            vec![source.clone(), source.clone()],
            5,
            PromptPolicy::SingleVerse,
            &mut rng,
        )
        .unwrap();

        assert_eq!(rounds.len(), 1);
        assert_eq!(rounds[0].source_passage, source);
        assert_eq!(rounds[0].passage.verses.len(), 1);
        assert!(source.verses.contains(&rounds[0].passage.verses[0]));
    }

    #[test]
    fn random_rounds_stay_inside_the_scope() {
        let library = ScriptureLibrary::standard_works().unwrap();
        let scope = GameMode::Bible.scope();
        let playable = library.playable_verse_count(Difficulty::Normal, &scope);
        let mut rng = SmallRng::seed_from_u64(3);

        let rounds =
            random_rounds(&library, Difficulty::Normal, &scope, 20, playable, &mut rng).unwrap();

        assert_eq!(rounds.len(), 20);
        assert!(
            rounds
                .iter()
                .all(|round| scope.includes_book(round.passage.canon, &round.passage.book))
        );
    }
}
