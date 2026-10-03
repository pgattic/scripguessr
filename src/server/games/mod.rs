mod rounds;
mod store;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use rand::SeedableRng;
use rand::rngs::SmallRng;
use sqlx::PgExecutor;
use tower_sessions::Session;
use uuid::Uuid;

use self::store::{DbGame, GameKind, Lock, NewGame};
use super::validation::{validate_game_basics, validate_passages};
use super::{ApiError, AppState, auth};
use crate::api::{
    AdvanceGameResponse, GameSnapshotResponse, GuessRequest, GuessResponse,
    LEADERBOARD_ROUND_COUNTS, NewGameRequest, ScopeSummary, SnapshotRound,
};
use crate::scoring::max_total_score;
use crate::scriptures::GameMode;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/games", post(create_game))
        .route("/api/games/{game_id}", get(game_snapshot))
        .route("/api/games/{game_id}/guesses", post(submit_guess))
        .route("/api/games/{game_id}/advance", post(advance_game))
}

async fn create_game(
    State(state): State<AppState>,
    session: Session,
    Json(request): Json<NewGameRequest>,
) -> Result<Json<GameSnapshotResponse>, ApiError> {
    let game = plan_game(&state, request)?;
    let id = Uuid::new_v4();
    let user_id = auth::optional_user_id(&session).await?;
    let mut tx = state.pool.begin().await?;
    store::insert_game(&mut tx, id, user_id, &game).await?;
    tx.commit().await?;

    let round_count = game.rounds.len();
    Ok(Json(GameSnapshotResponse {
        game_id: id.to_string(),
        rounds: game
            .rounds
            .into_iter()
            .map(|round| SnapshotRound {
                text: round.text,
                guess: None,
            })
            .collect(),
        current_round_index: 0,
        finished: false,
        summary: ScopeSummary {
            playable_verse_count: game.playable_verse_count,
            ..state.library.summary(game.difficulty, game.scope)
        },
        max_total_score: max_total_score(round_count),
    }))
}

fn plan_game(state: &AppState, request: NewGameRequest) -> Result<NewGame, ApiError> {
    let mut rng = SmallRng::from_os_rng();
    match request {
        NewGameRequest::Random {
            round_count,
            difficulty,
            scope,
        } => {
            validate_game_basics(round_count, &scope)?;
            let playable_verse_count = state.library.playable_verse_count(difficulty, &scope);
            let rounds = rounds::random_rounds(
                &state.library,
                difficulty,
                &scope,
                round_count,
                playable_verse_count,
                &mut rng,
            )?;
            Ok(NewGame {
                kind: GameKind::Random,
                leaderboard_preset: leaderboard_preset(&scope, rounds.len()),
                difficulty,
                scope,
                playable_verse_count,
                rounds,
            })
        }
        NewGameRequest::Study {
            round_count,
            difficulty,
            scope,
            passages,
            prompt_policy,
        } => {
            validate_game_basics(round_count, &scope)?;
            if passages.is_empty() {
                return Err(ApiError::bad_request("Study game must include passages"));
            }
            validate_passages(&state.library, &scope, &passages)?;
            let rounds = rounds::study_rounds(
                &state.library,
                passages,
                round_count,
                prompt_policy,
                &mut rng,
            )?;
            Ok(NewGame {
                kind: GameKind::Study,
                leaderboard_preset: None,
                difficulty,
                scope,
                playable_verse_count: rounds.len(),
                rounds,
            })
        }
    }
}

async fn game_snapshot(
    State(state): State<AppState>,
    session: Session,
    Path(game_id): Path<String>,
) -> Result<Json<GameSnapshotResponse>, ApiError> {
    let (id, game) = authorized_game(&state.pool, &session, &game_id, Lock::None).await?;
    let rounds = store::fetch_rounds(&state.pool, id).await?;
    store::touch_game(&state.pool, id).await?;

    Ok(Json(GameSnapshotResponse {
        game_id,
        rounds: rounds
            .into_iter()
            .map(|round| SnapshotRound {
                text: round.text,
                guess: round.guess.map(|guess| guess.0),
            })
            .collect(),
        current_round_index: game.current_round_index as usize,
        finished: game.finished,
        summary: ScopeSummary {
            playable_verse_count: game.playable_verse_count as usize,
            ..state.library.summary(game.difficulty.0, game.scope.0)
        },
        max_total_score: max_total_score(game.round_count as usize),
    }))
}

async fn submit_guess(
    State(state): State<AppState>,
    session: Session,
    Path(game_id): Path<String>,
    Json(request): Json<GuessRequest>,
) -> Result<Json<GuessResponse>, ApiError> {
    let mut tx = state.pool.begin().await?;
    let (id, game) = authorized_game(&mut *tx, &session, &game_id, Lock::ForUpdate).await?;
    let round_index = game.current_round_index;
    if game.finished || request.round_index != round_index as usize {
        return Err(ApiError::bad_request("Round is not active"));
    }
    let round = store::fetch_round(&mut *tx, id, round_index)
        .await?
        .ok_or_else(|| ApiError::bad_request("Round not found"))?;
    if let Some(guess) = round.guess {
        return Ok(Json(guess.0));
    }
    let answer = round.passage.0;
    if answer.verses.is_empty() {
        return Err(ApiError::bad_request("Round has no answer verses"));
    }
    let answer_chapter = answer.chapter_ref();
    let response = GuessResponse {
        score: state
            .library
            .score(&game.scope.0, &answer_chapter, &request.chapter),
        chapter_verses: state.library.chapter_verses(&answer_chapter),
        answer,
        source_passage: round.source_passage.0,
        guess: request.chapter,
    };
    store::save_guess(&mut *tx, id, round_index, &response).await?;
    store::touch_game(&mut *tx, id).await?;
    tx.commit().await?;
    Ok(Json(response))
}

async fn advance_game(
    State(state): State<AppState>,
    session: Session,
    Path(game_id): Path<String>,
) -> Result<Json<AdvanceGameResponse>, ApiError> {
    let mut tx = state.pool.begin().await?;
    let (id, game) = authorized_game(&mut *tx, &session, &game_id, Lock::ForUpdate).await?;
    let current = game.current_round_index;
    if game.finished {
        return Ok(Json(AdvanceGameResponse {
            current_round_index: current as usize,
            finished: true,
        }));
    }
    if !store::round_guessed(&mut *tx, id, current).await? {
        return Err(ApiError::bad_request("Submit a guess before advancing"));
    }

    let response = if current + 1 == game.round_count {
        store::finish_game(&mut *tx, id).await?;
        AdvanceGameResponse {
            current_round_index: current as usize,
            finished: true,
        }
    } else {
        store::set_round_index(&mut *tx, id, current + 1).await?;
        AdvanceGameResponse {
            current_round_index: (current + 1) as usize,
            finished: false,
        }
    };
    tx.commit().await?;
    Ok(Json(response))
}

/// Loads a game the session may play: anonymous games are open to anyone holding the id.
async fn authorized_game<'e>(
    conn: impl PgExecutor<'e>,
    session: &Session,
    game_id: &str,
    lock: Lock,
) -> Result<(Uuid, DbGame), ApiError> {
    let id = Uuid::parse_str(game_id).map_err(|_| store::game_not_found())?;
    let game = store::fetch_game(conn, id, lock).await?;
    if game.user_id.is_some() && game.user_id != auth::optional_user_id(session).await? {
        return Err(ApiError::unauthorized(
            "This game belongs to another account",
        ));
    }
    Ok((id, game))
}

fn leaderboard_preset(
    scope: &crate::scriptures::GameScope,
    round_count: usize,
) -> Option<&'static str> {
    LEADERBOARD_ROUND_COUNTS
        .contains(&round_count)
        .then(|| GameMode::matching(scope))
        .flatten()
        .map(GameMode::key)
}
