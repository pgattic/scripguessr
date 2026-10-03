use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use sqlx::FromRow;
use sqlx::types::Json as DbJson;
use tower_sessions::Session;
use uuid::Uuid;

use super::{ApiError, AppState, auth};
use crate::api::{
    LEADERBOARD_ROUND_COUNTS, LeaderboardEntry, LeaderboardQuery, LeaderboardResponse,
};

const LEADERBOARD_SIZE: i64 = 50;

#[derive(FromRow)]
struct DbLeaderboardRow {
    rank: i64,
    user_id: Uuid,
    username: String,
    score: i32,
    possible_score: i32,
    completed_at: time::OffsetDateTime,
}

pub(super) fn routes() -> Router<AppState> {
    Router::new().route("/api/leaderboards", get(leaderboard))
}

async fn leaderboard(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<LeaderboardQuery>,
) -> Result<Json<LeaderboardResponse>, ApiError> {
    if !LEADERBOARD_ROUND_COUNTS.contains(&query.rounds) {
        return Err(ApiError::bad_request("Leaderboard rounds must be 5 or 10"));
    }
    let current_user = auth::optional_user_id(&session).await?;
    let rows = sqlx::query_as::<_, DbLeaderboardRow>(
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
        WHERE rank <= $4 OR user_id = $5
        ORDER BY rank
        "#,
    )
    .bind(query.preset.key())
    .bind(DbJson(query.difficulty))
    .bind(query.rounds as i32)
    .bind(LEADERBOARD_SIZE)
    .bind(current_user)
    .fetch_all(&state.pool)
    .await?;

    let mut current_user_entry = None;
    let mut entries = Vec::new();
    for row in rows {
        let entry = LeaderboardEntry {
            rank: row.rank as u32,
            username: row.username,
            score: row.score as u32,
            possible_score: row.possible_score as u32,
            completed_at: row.completed_at.date().to_string(),
            current_user: Some(row.user_id) == current_user,
        };
        if row.rank <= LEADERBOARD_SIZE {
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
