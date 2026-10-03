use std::collections::HashMap;

use axum::extract::State;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use sqlx::FromRow;
use sqlx::types::Json as DbJson;
use tower_sessions::Session;
use uuid::Uuid;

use super::validation::validate_passages;
use super::{ApiError, AppState, auth};
use crate::api::{AccountDataResponse, GuessResponse, IdRequest};
use crate::scoring::MAX_SCORE;
use crate::scriptures::Difficulty;
use crate::stats::{FinishedGame, FinishedRound, ReviewItem, Stats};
use crate::study_sets::{PromptPolicy, StudyGuessScope, StudyPassage, StudySet};

const MAX_STUDY_SET_NAME_BYTES: usize = 100;

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

#[derive(FromRow)]
struct DbFinishedGame {
    id: Uuid,
    difficulty: DbJson<Difficulty>,
    score: i32,
    possible_score: i32,
}

#[derive(FromRow)]
struct DbGuess {
    game_id: Uuid,
    guess: DbJson<GuessResponse>,
}

async fn load_stats(state: &AppState, user_id: Uuid) -> Result<Stats, ApiError> {
    let games = sqlx::query_as::<_, DbFinishedGame>(
        "SELECT id, difficulty, score, possible_score FROM games WHERE user_id = $1 AND finished ORDER BY completed_at",
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await?;
    let guesses = sqlx::query_as::<_, DbGuess>(
        "SELECT r.game_id, r.guess FROM game_rounds r JOIN games g ON g.id = r.game_id WHERE g.user_id = $1 AND g.finished AND r.guess IS NOT NULL ORDER BY r.round_index",
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await?;
    let mut rounds_by_game = HashMap::<Uuid, Vec<FinishedRound>>::new();
    for row in guesses {
        let guess = row.guess.0;
        rounds_by_game
            .entry(row.game_id)
            .or_default()
            .push(FinishedRound {
                answer_book: guess.answer.book,
                score: guess.score.points,
                possible_score: MAX_SCORE,
            });
    }

    let mut stats = Stats::default();
    for game in games {
        stats.record_game(FinishedGame {
            difficulty: game.difficulty.0,
            score: game.score as u32,
            possible_score: game.possible_score as u32,
            rounds: rounds_by_game.remove(&game.id).unwrap_or_default(),
        });
    }
    Ok(stats)
}

#[derive(FromRow)]
struct DbReviewItem {
    passage: DbJson<StudyPassage>,
    text: String,
    score: i32,
}

impl From<DbReviewItem> for ReviewItem {
    fn from(row: DbReviewItem) -> Self {
        Self {
            passage: row.passage.0,
            text: row.text,
            score: row.score as u32,
        }
    }
}

async fn load_review_items(state: &AppState, user_id: Uuid) -> Result<Vec<ReviewItem>, ApiError> {
    let rows = sqlx::query_as::<_, DbReviewItem>(
        "SELECT passage, text, score FROM review_items WHERE user_id = $1 ORDER BY created_at",
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(rows.into_iter().map(ReviewItem::from).collect())
}

async fn find_review_item(
    state: &AppState,
    user_id: Uuid,
    passage: &StudyPassage,
) -> Result<Option<Uuid>, ApiError> {
    Ok(sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM review_items WHERE user_id = $1 AND passage = $2 LIMIT 1",
    )
    .bind(user_id)
    .bind(DbJson(passage))
    .fetch_optional(&state.pool)
    .await?)
}

async fn save_review_item(
    State(state): State<AppState>,
    session: Session,
    Json(item): Json<ReviewItem>,
) -> Result<Json<()>, ApiError> {
    let user_id = auth::require_user_id(&session).await?;
    if let Some(id) = find_review_item(&state, user_id, &item.passage).await? {
        sqlx::query("UPDATE review_items SET text = $1, score = $2 WHERE id = $3")
            .bind(item.text)
            .bind(item.score as i32)
            .bind(id)
            .execute(&state.pool)
            .await?;
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
        .await?;
    }
    Ok(Json(()))
}

async fn remove_review_item(
    State(state): State<AppState>,
    session: Session,
    Json(passage): Json<StudyPassage>,
) -> Result<Json<()>, ApiError> {
    let user_id = auth::require_user_id(&session).await?;
    if let Some(id) = find_review_item(&state, user_id, &passage).await? {
        sqlx::query("DELETE FROM review_items WHERE id = $1")
            .bind(id)
            .execute(&state.pool)
            .await?;
    }
    Ok(Json(()))
}

#[derive(FromRow)]
struct DbStudySet {
    id: Uuid,
    name: String,
    passages: DbJson<Vec<StudyPassage>>,
    guess_scope: DbJson<StudyGuessScope>,
    prompt_policy: DbJson<PromptPolicy>,
}

impl From<DbStudySet> for StudySet {
    fn from(row: DbStudySet) -> Self {
        Self {
            id: row.id.to_string(),
            name: row.name,
            passages: row.passages.0,
            guess_scope: row.guess_scope.0,
            prompt_policy: row.prompt_policy.0,
        }
    }
}

async fn load_study_sets(state: &AppState, user_id: Uuid) -> Result<Vec<StudySet>, ApiError> {
    let rows = sqlx::query_as::<_, DbStudySet>(
        "SELECT id, name, passages, guess_scope, prompt_policy FROM custom_study_sets WHERE user_id = $1 ORDER BY created_at",
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(rows.into_iter().map(StudySet::from).collect())
}

async fn save_study_set(
    State(state): State<AppState>,
    session: Session,
    Json(mut set): Json<StudySet>,
) -> Result<Json<StudySet>, ApiError> {
    let user_id = auth::require_user_id(&session).await?;
    set.name = set.name.trim().to_string();
    if set.name.is_empty() || set.name.len() > MAX_STUDY_SET_NAME_BYTES {
        return Err(ApiError::bad_request(
            "Study set name must be 1-100 characters",
        ));
    }
    validate_passages(&state.library, &set.resolved_guess_scope(), &set.passages)?;
    let id = Uuid::parse_str(&set.id).unwrap_or_else(|_| Uuid::new_v4());
    let updated = sqlx::query(
        "UPDATE custom_study_sets SET name = $1, passages = $2, guess_scope = $3, prompt_policy = $4, updated_at = now() WHERE id = $5 AND user_id = $6",
    )
    .bind(&set.name)
    .bind(DbJson(&set.passages))
    .bind(DbJson(&set.guess_scope))
    .bind(DbJson(set.prompt_policy))
    .bind(id)
    .bind(user_id)
    .execute(&state.pool)
    .await?;
    if updated.rows_affected() == 0 {
        sqlx::query(
            "INSERT INTO custom_study_sets (id, user_id, name, passages, guess_scope, prompt_policy) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(id)
        .bind(user_id)
        .bind(&set.name)
        .bind(DbJson(&set.passages))
        .bind(DbJson(&set.guess_scope))
        .bind(DbJson(set.prompt_policy))
        .execute(&state.pool)
        .await?;
    }
    set.id = id.to_string();
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
        .await?;
    Ok(Json(()))
}
