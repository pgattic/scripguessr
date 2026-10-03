use super::ApiError;
use crate::scriptures::{GameScope, ScriptureLibrary};
use crate::study_sets::{MAX_VERSES_PER_PASSAGE, StudyPassage};

const MAX_GAME_ROUNDS: usize = 1_000;
const MAX_STUDY_PASSAGES: usize = 2_000;

pub fn validate_game_basics(round_count: usize, scope: &GameScope) -> Result<(), ApiError> {
    if scope.canons.is_empty() || round_count == 0 {
        return Err(ApiError::bad_request("Game must include rounds and scope"));
    }
    if round_count > MAX_GAME_ROUNDS {
        return Err(ApiError::bad_request("Too many rounds requested"));
    }
    Ok(())
}

/// Checks that every passage exists and lies inside `scope`. An empty list is valid.
pub fn validate_passages(
    library: &ScriptureLibrary,
    scope: &GameScope,
    passages: &[StudyPassage],
) -> Result<(), ApiError> {
    if passages.len() > MAX_STUDY_PASSAGES {
        return Err(ApiError::bad_request("Too many study passages"));
    }
    for passage in passages {
        if !scope.includes_book(passage.canon, &passage.book) {
            return Err(ApiError::bad_request(
                "Study passage is outside the game scope",
            ));
        }
        if passage.verses.is_empty() || passage.verses.len() > MAX_VERSES_PER_PASSAGE {
            return Err(ApiError::bad_request(
                "Study passage has an invalid verse count",
            ));
        }
        let exists = library.scriptures(passage.canon).is_some_and(|scriptures| {
            scriptures.contains_passage(&passage.book, passage.chapter, &passage.verses)
        });
        if !exists {
            return Err(ApiError::bad_request("Study passage was not found"));
        }
    }
    Ok(())
}
