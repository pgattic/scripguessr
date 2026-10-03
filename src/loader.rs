use serde::Serialize;
use serde::de::DeserializeOwned;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

use crate::api::{
    AccountDataResponse, AdvanceGameResponse, AuthRequest, ChangePasswordRequest, ChapterResponse,
    GameSnapshotResponse, GuessRequest, GuessResponse, IdRequest, LeaderboardQuery,
    LeaderboardResponse, MetadataRequest, NewGameRequest, ScopeSummary, UserResponse,
};
use crate::scriptures::ChapterRef;
use crate::stats::ReviewItem;
use crate::study_sets::{StudyPassage, StudySet};

const DEV_SERVER_HINT: &str =
    "In development, use `nix run .#dev` so the backend starts with the Dioxus dev server.";

pub async fn load_metadata(request: MetadataRequest) -> Result<ScopeSummary, String> {
    post("/api/metadata", &request).await
}

pub async fn create_game(request: NewGameRequest) -> Result<GameSnapshotResponse, String> {
    post("/api/games", &request).await
}

pub async fn load_game(game_id: &str) -> Result<GameSnapshotResponse, String> {
    get(&format!("/api/games/{game_id}")).await
}

pub async fn submit_guess(game_id: &str, request: GuessRequest) -> Result<GuessResponse, String> {
    post(&format!("/api/games/{game_id}/guesses"), &request).await
}

pub async fn advance_game(game_id: &str) -> Result<AdvanceGameResponse, String> {
    post(&format!("/api/games/{game_id}/advance"), &()).await
}

pub async fn load_chapter(chapter: ChapterRef) -> Result<ChapterResponse, String> {
    post("/api/chapter", &chapter).await
}

pub async fn current_user() -> Result<Option<UserResponse>, String> {
    let path = "/api/me";
    let response = send("GET", path, None::<&()>).await?;
    if response.status() == 401 {
        return Ok(None);
    }
    parse_response(path, response).await.map(Some)
}

pub async fn register(request: AuthRequest) -> Result<UserResponse, String> {
    post("/api/auth/register", &request).await
}

pub async fn login(request: AuthRequest) -> Result<UserResponse, String> {
    post("/api/auth/login", &request).await
}

pub async fn logout() -> Result<(), String> {
    post("/api/auth/logout", &()).await
}

pub async fn change_password(request: ChangePasswordRequest) -> Result<(), String> {
    post("/api/auth/change-password", &request).await
}

pub async fn load_account_data() -> Result<AccountDataResponse, String> {
    get("/api/me/data").await
}

pub async fn save_review_item(item: ReviewItem) -> Result<(), String> {
    put("/api/me/review-items", &item).await
}

pub async fn remove_review_item(passage: StudyPassage) -> Result<(), String> {
    post("/api/me/review-items/remove", &passage).await
}

pub async fn save_study_set(set: StudySet) -> Result<StudySet, String> {
    put("/api/me/study-sets", &set).await
}

pub async fn remove_study_set(id: String) -> Result<(), String> {
    post("/api/me/study-sets/remove", &IdRequest { id }).await
}

pub async fn load_leaderboard(query: LeaderboardQuery) -> Result<LeaderboardResponse, String> {
    get(&format!("/api/leaderboards?{}", query_string(&query)?)).await
}

async fn get<Response: DeserializeOwned>(path: &str) -> Result<Response, String> {
    request("GET", path, None::<&()>).await
}

async fn post<Response: DeserializeOwned>(
    path: &str,
    body: &impl Serialize,
) -> Result<Response, String> {
    request("POST", path, Some(body)).await
}

async fn put<Response: DeserializeOwned>(
    path: &str,
    body: &impl Serialize,
) -> Result<Response, String> {
    request("PUT", path, Some(body)).await
}

async fn request<Response: DeserializeOwned>(
    method: &str,
    path: &str,
    body: Option<&impl Serialize>,
) -> Result<Response, String> {
    let response = send(method, path, body).await?;
    parse_response(path, response).await
}

async fn send(
    method: &str,
    path: &str,
    body: Option<&impl Serialize>,
) -> Result<web_sys::Response, String> {
    let url = api_url(path);
    let window = web_sys::window().ok_or_else(|| "Browser window is not available".to_string())?;
    let options = web_sys::RequestInit::new();
    options.set_method(method);
    options.set_credentials(web_sys::RequestCredentials::Include);
    if let Some(body) = body {
        let json = serde_json::to_string(body)
            .map_err(|error| format!("Could not serialize request: {error}"))?;
        let headers = web_sys::Headers::new()
            .map_err(|error| format!("Could not create request headers: {error:?}"))?;
        headers
            .set("Content-Type", "application/json")
            .map_err(|error| format!("Could not set request headers: {error:?}"))?;
        options.set_body(&JsValue::from_str(&json));
        options.set_headers(&headers);
    }

    JsFuture::from(window.fetch_with_str_and_init(&url, &options))
        .await
        .map_err(|error| {
            format!(
                "Could not reach the game server at {url}. {DEV_SERVER_HINT} Details: {error:?}"
            )
        })?
        .dyn_into::<web_sys::Response>()
        .map_err(|error| format!("Invalid response for {url}: {error:?}"))
}

async fn parse_response<Response: DeserializeOwned>(
    path: &str,
    response: web_sys::Response,
) -> Result<Response, String> {
    let text = JsFuture::from(
        response
            .text()
            .map_err(|error| format!("Could not read {path}: {error:?}"))?,
    )
    .await
    .map_err(|error| format!("Could not read {path}: {error:?}"))?
    .as_string()
    .ok_or_else(|| format!("Could not read {path} as text"))?;

    if !response.ok() {
        return Err(error_message(&text).unwrap_or_else(|| {
            if response.status() == 405 {
                format!("The game server did not accept {path}. {DEV_SERVER_HINT}")
            } else {
                format!(
                    "The game server returned HTTP {} for {path}",
                    response.status()
                )
            }
        }));
    }

    serde_json::from_str(&text).map_err(|error| format!("Could not parse {path}: {error}"))
}

fn api_url(path: &str) -> String {
    let base = option_env!("SCRIPGUESSR_API_BASE").unwrap_or_default();
    format!("{}{path}", base.trim_end_matches('/'))
}

fn query_string(query: &impl Serialize) -> Result<String, String> {
    let serde_json::Value::Object(fields) = serde_json::to_value(query)
        .map_err(|error| format!("Could not serialize query: {error}"))?
    else {
        return Err("Query must serialize to an object".to_string());
    };
    Ok(fields
        .into_iter()
        .map(|(key, value)| {
            let value = match value {
                serde_json::Value::String(text) => text,
                other => other.to_string(),
            };
            format!("{key}={}", js_sys::encode_uri_component(&value))
        })
        .collect::<Vec<_>>()
        .join("&"))
}

fn error_message(text: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()?
        .get("error")?
        .as_str()
        .map(ToString::to_string)
}
