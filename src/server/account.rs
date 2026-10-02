use axum::extract::State;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use sqlx::types::Json as DbJson;
use tower_sessions::Session;
use uuid::Uuid;

use super::{ApiError, AppState, auth};
use crate::api::{AccountDataResponse, GuessResponse, IdRequest};
use crate::stats::{FinishedGame, FinishedRound, ReviewItem, Stats};
use crate::study_sets::{PromptPolicy, StudyGuessScope, StudyPassage, StudySet};

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/me/data", get(data))
        .route("/api/me/review-items", put(save_review_item))
        .route("/api/me/review-items/remove", post(remove_review_item))
        .route("/api/me/study-sets", put(save_study_set))
        .route("/api/me/study-sets/remove", post(remove_study_set))
}

async fn data(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<AccountDataResponse>, ApiError> {
    let user_id = auth::require_user_id(&session).await?;
    Ok(Json(AccountDataResponse {
        stats: load_stats(&state, user_id).await?,
        review_items: load_review_items(&state, user_id).await?,
        custom_study_sets: load_study_sets(&state, user_id).await?,
    }))
}

async fn load_stats(state: &AppState, user_id: Uuid) -> Result<Stats, ApiError> {
    let games = sqlx::query_as::<_, (Uuid, DbJson<crate::scriptures::Difficulty>, i32, i32)>(
        "SELECT id, difficulty, score, possible_score FROM games WHERE user_id = $1 AND finished ORDER BY completed_at",
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(ApiError::database)?;
    let mut stats = Stats::default();
    for (game_id, difficulty, score, possible_score) in games {
        let guesses = sqlx::query_scalar::<_, DbJson<GuessResponse>>(
            "SELECT guess FROM game_rounds WHERE game_id = $1 AND guess IS NOT NULL ORDER BY round_index",
        )
        .bind(game_id)
        .fetch_all(&state.pool)
        .await
        .map_err(ApiError::database)?;
        stats.record_game(FinishedGame {
            difficulty: difficulty.0,
            score: score as u32,
            possible_score: possible_score as u32,
            rounds: guesses
                .into_iter()
                .map(|guess| {
                    let guess = guess.0;
                    FinishedRound {
                        answer_book: guess.answer.book,
                        score: guess.score.points,
                        possible_score: crate::scoring::MAX_SCORE,
                    }
                })
                .collect(),
        });
    }
    Ok(stats)
}

async fn load_review_items(state: &AppState, user_id: Uuid) -> Result<Vec<ReviewItem>, ApiError> {
    let rows = sqlx::query_as::<_, (DbJson<StudyPassage>, String, i32)>(
        "SELECT passage, text, score FROM review_items WHERE user_id = $1 ORDER BY created_at",
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(ApiError::database)?;
    Ok(rows
        .into_iter()
        .map(|(passage, text, score)| ReviewItem {
            passage: passage.0,
            text,
            score: score as u32,
        })
        .collect())
}

async fn save_review_item(
    State(state): State<AppState>,
    session: Session,
    Json(item): Json<ReviewItem>,
) -> Result<Json<()>, ApiError> {
    let user_id = auth::require_user_id(&session).await?;
    let passages = sqlx::query_as::<_, (Uuid, DbJson<StudyPassage>)>(
        "SELECT id, passage FROM review_items WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(ApiError::database)?;
    if let Some((id, _)) = passages
        .into_iter()
        .find(|(_, passage)| passage.0 == item.passage)
    {
        sqlx::query("UPDATE review_items SET text = $1, score = $2 WHERE id = $3")
            .bind(item.text)
            .bind(item.score as i32)
            .bind(id)
            .execute(&state.pool)
            .await
            .map_err(ApiError::database)?;
    } else {
        sqlx::query(
            "INSERT INTO review_items (id, user_id, passage, text, score) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(DbJson(item.passage))
        .bind(item.text)
        .bind(item.score as i32)
        .execute(&state.pool)
        .await
        .map_err(ApiError::database)?;
    }
    Ok(Json(()))
}

async fn remove_review_item(
    State(state): State<AppState>,
    session: Session,
    Json(passage): Json<StudyPassage>,
) -> Result<Json<()>, ApiError> {
    let user_id = auth::require_user_id(&session).await?;
    let rows = sqlx::query_as::<_, (Uuid, DbJson<StudyPassage>)>(
        "SELECT id, passage FROM review_items WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(ApiError::database)?;
    if let Some((id, _)) = rows.into_iter().find(|(_, stored)| stored.0 == passage) {
        sqlx::query("DELETE FROM review_items WHERE id = $1")
            .bind(id)
            .execute(&state.pool)
            .await
            .map_err(ApiError::database)?;
    }
    Ok(Json(()))
}

async fn load_study_sets(state: &AppState, user_id: Uuid) -> Result<Vec<StudySet>, ApiError> {
    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            DbJson<Vec<StudyPassage>>,
            DbJson<StudyGuessScope>,
            DbJson<PromptPolicy>,
        ),
    >(
        "SELECT id, name, passages, guess_scope, prompt_policy FROM custom_study_sets WHERE user_id = $1 ORDER BY created_at",
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(ApiError::database)?;
    Ok(rows
        .into_iter()
        .map(
            |(id, name, passages, guess_scope, prompt_policy)| StudySet {
                id: id.to_string(),
                name,
                passages: passages.0,
                guess_scope: guess_scope.0,
                prompt_policy: prompt_policy.0,
            },
        )
        .collect())
}

async fn save_study_set(
    State(state): State<AppState>,
    session: Session,
    Json(mut set): Json<StudySet>,
) -> Result<Json<StudySet>, ApiError> {
    let user_id = auth::require_user_id(&session).await?;
    if set.name.trim().is_empty() || set.name.len() > 100 {
        return Err(ApiError::bad_request(
            "Study set name must be 1-100 characters",
        ));
    }
    if set.passages.len() > super::MAX_STUDY_PASSAGES {
        return Err(ApiError::bad_request("Study set has too many passages"));
    }
    super::validate_study_passages(&state.library, &set.resolved_guess_scope(), &set.passages)?;
    let id = Uuid::parse_str(&set.id).unwrap_or_else(|_| Uuid::new_v4());
    let updated = sqlx::query(
        "UPDATE custom_study_sets SET name = $1, passages = $2, guess_scope = $3, prompt_policy = $4, updated_at = now() WHERE id = $5 AND user_id = $6",
    )
    .bind(set.name.trim())
    .bind(DbJson(&set.passages))
    .bind(DbJson(&set.guess_scope))
    .bind(DbJson(set.prompt_policy))
    .bind(id)
    .bind(user_id)
    .execute(&state.pool)
    .await
    .map_err(ApiError::database)?;
    if updated.rows_affected() == 0 {
        sqlx::query(
            "INSERT INTO custom_study_sets (id, user_id, name, passages, guess_scope, prompt_policy) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(id)
        .bind(user_id)
        .bind(set.name.trim())
        .bind(DbJson(&set.passages))
        .bind(DbJson(&set.guess_scope))
        .bind(DbJson(set.prompt_policy))
        .execute(&state.pool)
        .await
        .map_err(ApiError::database)?;
    }
    set.id = id.to_string();
    set.name = set.name.trim().to_string();
    Ok(Json(set))
}

async fn remove_study_set(
    State(state): State<AppState>,
    session: Session,
    Json(request): Json<IdRequest>,
) -> Result<Json<()>, ApiError> {
    let user_id = auth::require_user_id(&session).await?;
    let Ok(id) = Uuid::parse_str(&request.id) else {
        return Ok(Json(()));
    };
    sqlx::query("DELETE FROM custom_study_sets WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(&state.pool)
        .await
        .map_err(ApiError::database)?;
    Ok(Json(()))
}
