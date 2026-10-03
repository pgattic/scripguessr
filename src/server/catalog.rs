use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};

use super::{ApiError, AppState};
use crate::api::{ChapterResponse, MetadataRequest, ScopeSummary};
use crate::scriptures::ChapterRef;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/api/metadata", post(metadata))
        .route("/api/chapter", post(chapter))
}

async fn healthz() -> &'static str {
    "ok"
}

async fn readyz(State(state): State<AppState>) -> Result<&'static str, ApiError> {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pool)
        .await?;
    Ok("ready")
}

async fn metadata(
    State(state): State<AppState>,
    Json(request): Json<MetadataRequest>,
) -> Result<Json<ScopeSummary>, ApiError> {
    if request.scope.canons.is_empty() {
        return Err(ApiError::bad_request(
            "At least one scope item must be selected",
        ));
    }
    Ok(Json(
        state.library.summary(request.difficulty, request.scope),
    ))
}

async fn chapter(
    State(state): State<AppState>,
    Json(chapter): Json<ChapterRef>,
) -> Result<Json<ChapterResponse>, ApiError> {
    let verses = state.library.chapter_verses(&chapter);
    if verses.is_empty() {
        return Err(ApiError::not_found("Chapter not found"));
    }
    Ok(Json(ChapterResponse { verses }))
}
