use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use tower_sessions::Session;
use uuid::Uuid;

use super::{ApiError, AppState, auth, game_mode_key};
use crate::api::{LeaderboardEntry, LeaderboardResponse};
use crate::scriptures::{Difficulty, GameMode};

#[derive(Deserialize)]
struct LeaderboardQuery {
    preset: GameMode,
    difficulty: Difficulty,
    rounds: usize,
}

pub(super) fn routes() -> Router<AppState> {
    Router::new().route("/api/leaderboards", get(leaderboard))
}

async fn leaderboard(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<LeaderboardQuery>,
) -> Result<Json<LeaderboardResponse>, ApiError> {
    if !matches!(query.rounds, 5 | 10) {
        return Err(ApiError::bad_request("Leaderboard rounds must be 5 or 10"));
    }
    let current_user = auth::optional_user_id(&session).await?;
    let rows = sqlx::query_as::<_, (i64, Uuid, String, i32, i32, time::OffsetDateTime)>(
        r#"
        WITH ranked AS (
            SELECT
                g.user_id,
                u.username,
                g.score,
                g.possible_score,
                g.completed_at,
                row_number() OVER (
                    PARTITION BY g.user_id
                    ORDER BY g.score DESC, g.completed_at ASC
                ) AS user_attempt
            FROM games g
            JOIN users u ON u.id = g.user_id
            WHERE g.finished
              AND g.leaderboard_preset = $1
              AND g.difficulty = $2
              AND g.round_count = $3
        ), best AS (
            SELECT *, row_number() OVER (ORDER BY score DESC, completed_at ASC) AS rank
            FROM ranked
            WHERE user_attempt = 1
        )
        SELECT rank, user_id, username, score, possible_score, completed_at
        FROM best
        WHERE rank <= 50 OR user_id = $4
        ORDER BY rank
        "#,
    )
    .bind(game_mode_key(query.preset))
    .bind(sqlx::types::Json(query.difficulty))
    .bind(query.rounds as i32)
    .bind(current_user)
    .fetch_all(&state.pool)
    .await
    .map_err(ApiError::database)?;

    let mut current_user_entry = None;
    let mut entries = Vec::new();
    for (rank, user_id, username, score, possible_score, completed_at) in rows {
        let entry = LeaderboardEntry {
            rank: rank as u32,
            username,
            score: score as u32,
            possible_score: possible_score as u32,
            completed_at: completed_at.date().to_string(),
            current_user: Some(user_id) == current_user,
        };
        if entry.rank <= 50 {
            entries.push(entry.clone());
        }
        if entry.current_user {
            current_user_entry = Some(entry);
        }
    }
    Ok(Json(LeaderboardResponse {
        preset: query.preset,
        difficulty: query.difficulty,
        round_count: query.rounds,
        entries,
        current_user_entry,
    }))
}
