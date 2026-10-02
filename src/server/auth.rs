use argon2::Argon2;
use argon2::password_hash::{
    PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng,
};
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use tower_sessions::Session;
use uuid::Uuid;

use super::{ApiError, AppState};
use crate::api::{AuthRequest, ChangePasswordRequest, UserResponse};

const USER_ID_KEY: &str = "user_id";

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/change-password", post(change_password))
        .route("/api/me", get(me))
}

pub(super) async fn optional_user_id(session: &Session) -> Result<Option<Uuid>, ApiError> {
    let id = session
        .get::<String>(USER_ID_KEY)
        .await
        .map_err(ApiError::session)?;
    id.map(|id| Uuid::parse_str(&id).map_err(|_| ApiError::unauthorized("Invalid user session")))
        .transpose()
}

pub(super) async fn require_user_id(session: &Session) -> Result<Uuid, ApiError> {
    optional_user_id(session)
        .await?
        .ok_or_else(|| ApiError::unauthorized("Sign in required"))
}

async fn register(
    State(state): State<AppState>,
    session: Session,
    Json(request): Json<AuthRequest>,
) -> Result<Json<UserResponse>, ApiError> {
    let (username, normalized) = validate_username(&request.username)?;
    validate_password(&request.password)?;
    let password_hash = hash_password(request.password).await?;
    let id = Uuid::new_v4();

    let result = sqlx::query(
        "INSERT INTO users (id, username, username_normalized, password_hash) VALUES ($1, $2, $3, $4)",
    )
    .bind(id)
    .bind(&username)
    .bind(normalized)
    .bind(password_hash)
    .execute(&state.pool)
    .await;
    if let Err(error) = result {
        if error
            .as_database_error()
            .is_some_and(|error| error.is_unique_violation())
        {
            return Err(ApiError::conflict("Username is already taken"));
        }
        return Err(ApiError::database(error));
    }

    establish_session(&session, id).await?;
    Ok(Json(UserResponse {
        id: id.to_string(),
        username,
    }))
}

async fn login(
    State(state): State<AppState>,
    session: Session,
    Json(request): Json<AuthRequest>,
) -> Result<Json<UserResponse>, ApiError> {
    let normalized = normalize_username(&request.username);
    let row = sqlx::query_as::<_, (Uuid, String, String)>(
        "SELECT id, username, password_hash FROM users WHERE username_normalized = $1",
    )
    .bind(normalized)
    .fetch_optional(&state.pool)
    .await
    .map_err(ApiError::database)?
    .ok_or_else(|| ApiError::unauthorized("Invalid username or password"))?;

    if !verify_password(request.password, row.2).await? {
        return Err(ApiError::unauthorized("Invalid username or password"));
    }
    establish_session(&session, row.0).await?;
    Ok(Json(UserResponse {
        id: row.0.to_string(),
        username: row.1,
    }))
}

async fn logout(session: Session) -> Result<Json<()>, ApiError> {
    session.clear().await;
    Ok(Json(()))
}

async fn me(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<UserResponse>, ApiError> {
    let id = require_user_id(&session).await?;
    let username = sqlx::query_scalar::<_, String>("SELECT username FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(ApiError::database)?
        .ok_or_else(|| ApiError::unauthorized("User no longer exists"))?;
    Ok(Json(UserResponse {
        id: id.to_string(),
        username,
    }))
}

async fn change_password(
    State(state): State<AppState>,
    session: Session,
    Json(request): Json<ChangePasswordRequest>,
) -> Result<Json<()>, ApiError> {
    let id = require_user_id(&session).await?;
    validate_password(&request.new_password)?;
    let current_hash =
        sqlx::query_scalar::<_, String>("SELECT password_hash FROM users WHERE id = $1")
            .bind(id)
            .fetch_one(&state.pool)
            .await
            .map_err(ApiError::database)?;
    if !verify_password(request.current_password, current_hash).await? {
        return Err(ApiError::unauthorized("Current password is incorrect"));
    }
    let new_hash = hash_password(request.new_password).await?;
    sqlx::query("UPDATE users SET password_hash = $1, updated_at = now() WHERE id = $2")
        .bind(new_hash)
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(ApiError::database)?;
    session.cycle_id().await.map_err(ApiError::session)?;
    Ok(Json(()))
}

async fn establish_session(session: &Session, id: Uuid) -> Result<(), ApiError> {
    session.cycle_id().await.map_err(ApiError::session)?;
    session
        .insert(USER_ID_KEY, id.to_string())
        .await
        .map_err(ApiError::session)
}

fn validate_username(value: &str) -> Result<(String, String), ApiError> {
    let username = value.trim().to_string();
    if !(3..=32).contains(&username.len())
        || !username
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        return Err(ApiError::bad_request(
            "Username must be 3-32 letters, numbers, hyphens, or underscores",
        ));
    }
    let normalized = normalize_username(&username);
    Ok((username, normalized))
}

fn normalize_username(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn validate_password(value: &str) -> Result<(), ApiError> {
    if !(12..=128).contains(&value.len()) {
        return Err(ApiError::bad_request(
            "Password must be between 12 and 128 characters",
        ));
    }
    Ok(())
}

async fn hash_password(password: String) -> Result<String, ApiError> {
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
    })
    .await
    .map_err(|_| ApiError::internal("Password task failed"))?
    .map_err(|_| ApiError::internal("Could not hash password"))
}

async fn verify_password(password: String, encoded: String) -> Result<bool, ApiError> {
    tokio::task::spawn_blocking(move || {
        let Ok(hash) = PasswordHash::new(&encoded) else {
            return false;
        };
        Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok()
    })
    .await
    .map_err(|_| ApiError::internal("Password task failed"))
}
