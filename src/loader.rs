use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::api::{
    AccountDataResponse, AdvanceGameResponse, AuthRequest, ChangePasswordRequest, ChapterRequest,
    ChapterResponse, GameSnapshotResponse, GuessRequest, GuessResponse, IdRequest,
    LeaderboardResponse, MetadataRequest, MetadataResponse, NewGameRequest, NewGameResponse,
    UserResponse,
};
use crate::scriptures::{Difficulty, GameMode};
use crate::stats::ReviewItem;
use crate::study_sets::{StudyPassage, StudySet};

pub async fn load_metadata(request: MetadataRequest) -> Result<MetadataResponse, String> {
    post_json(&api_path("/api/metadata"), &request).await
}

pub async fn create_game(request: NewGameRequest) -> Result<NewGameResponse, String> {
    post_json(&api_path("/api/games"), &request).await
}

pub async fn load_game(game_id: &str) -> Result<GameSnapshotResponse, String> {
    get_json(&api_path(&format!("/api/games/{game_id}"))).await
}

pub async fn advance_game(game_id: &str) -> Result<AdvanceGameResponse, String> {
    post_json(&api_path(&format!("/api/games/{game_id}/advance")), &()).await
}

pub async fn current_user() -> Result<Option<UserResponse>, String> {
    get_optional_json(&api_path("/api/me")).await
}

pub async fn register(request: AuthRequest) -> Result<UserResponse, String> {
    post_json(&api_path("/api/auth/register"), &request).await
}

pub async fn login(request: AuthRequest) -> Result<UserResponse, String> {
    post_json(&api_path("/api/auth/login"), &request).await
}

pub async fn logout() -> Result<(), String> {
    post_json(&api_path("/api/auth/logout"), &()).await
}

pub async fn change_password(request: ChangePasswordRequest) -> Result<(), String> {
    post_json(&api_path("/api/auth/change-password"), &request).await
}

pub async fn load_account_data() -> Result<AccountDataResponse, String> {
    get_json(&api_path("/api/me/data")).await
}

pub async fn save_review_item(item: ReviewItem) -> Result<(), String> {
    put_json(&api_path("/api/me/review-items"), &item).await
}

pub async fn remove_review_item(passage: StudyPassage) -> Result<(), String> {
    post_json(&api_path("/api/me/review-items/remove"), &passage).await
}

pub async fn save_study_set(set: StudySet) -> Result<StudySet, String> {
    put_json(&api_path("/api/me/study-sets"), &set).await
}

pub async fn remove_study_set(id: String) -> Result<(), String> {
    post_json(&api_path("/api/me/study-sets/remove"), &IdRequest { id }).await
}

pub async fn load_leaderboard(
    preset: GameMode,
    difficulty: Difficulty,
    rounds: usize,
) -> Result<LeaderboardResponse, String> {
    get_json(&api_path(&format!(
        "/api/leaderboards?preset={preset:?}&difficulty={difficulty:?}&rounds={rounds}"
    )))
    .await
}

pub async fn load_chapter(request: ChapterRequest) -> Result<ChapterResponse, String> {
    post_json(&api_path("/api/chapter"), &request).await
}

pub async fn submit_guess(game_id: &str, request: GuessRequest) -> Result<GuessResponse, String> {
    post_json(
        &api_path(&format!("/api/games/{game_id}/guesses")),
        &request,
    )
    .await
}

fn api_path(path: &str) -> String {
    let base = option_env!("SCRIPGUESSR_API_BASE").unwrap_or_default();
    if base.is_empty() {
        path.to_string()
    } else {
        format!("{}{}", base.trim_end_matches('/'), path)
    }
}

#[cfg(target_arch = "wasm32")]
async fn get_optional_json<Response>(path: &str) -> Result<Option<Response>, String>
where
    Response: DeserializeOwned,
{
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let window = web_sys::window().ok_or_else(|| "Browser window is not available".to_string())?;
    let options = web_sys::RequestInit::new();
    options.set_credentials(web_sys::RequestCredentials::Include);
    let response_value = JsFuture::from(window.fetch_with_str_and_init(path, &options))
        .await
        .map_err(|error| format!("Could not reach the game server at {path}: {error:?}"))?;
    let response = response_value
        .dyn_into::<web_sys::Response>()
        .map_err(|error| format!("Invalid response for {path}: {error:?}"))?;
    if response.status() == 401 {
        return Ok(None);
    }
    parse_response(path, response).await.map(Some)
}

#[cfg(not(target_arch = "wasm32"))]
async fn get_optional_json<Response>(_path: &str) -> Result<Option<Response>, String>
where
    Response: DeserializeOwned,
{
    Err("Browser API client is only available in the web build".to_string())
}

#[cfg(target_arch = "wasm32")]
async fn get_json<Response>(path: &str) -> Result<Response, String>
where
    Response: DeserializeOwned,
{
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let window = web_sys::window().ok_or_else(|| "Browser window is not available".to_string())?;
    let options = web_sys::RequestInit::new();
    options.set_credentials(web_sys::RequestCredentials::Include);
    let response_value = JsFuture::from(window.fetch_with_str_and_init(path, &options))
        .await
        .map_err(|error| format!("Could not reach the game server at {path}: {error:?}"))?;
    let response = response_value
        .dyn_into::<web_sys::Response>()
        .map_err(|error| format!("Invalid response for {path}: {error:?}"))?;
    parse_response(path, response).await
}

#[cfg(not(target_arch = "wasm32"))]
async fn get_json<Response>(_path: &str) -> Result<Response, String>
where
    Response: DeserializeOwned,
{
    Err("Browser API client is only available in the web build".to_string())
}

#[cfg(target_arch = "wasm32")]
async fn post_json<Request, Response>(path: &str, body: &Request) -> Result<Response, String>
where
    Request: Serialize,
    Response: DeserializeOwned,
{
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;
    use wasm_bindgen_futures::JsFuture;

    let window = web_sys::window().ok_or_else(|| "Browser window is not available".to_string())?;
    let json = serde_json::to_string(body)
        .map_err(|error| format!("Could not serialize request: {error}"))?;

    let headers = web_sys::Headers::new()
        .map_err(|error| format!("Could not create request headers: {error:?}"))?;
    headers
        .set("Content-Type", "application/json")
        .map_err(|error| format!("Could not set request headers: {error:?}"))?;

    let options = web_sys::RequestInit::new();
    options.set_method("POST");
    options.set_body(&JsValue::from_str(&json));
    options.set_headers(&headers);
    options.set_credentials(web_sys::RequestCredentials::Include);

    let response_value = JsFuture::from(window.fetch_with_str_and_init(path, &options))
        .await
        .map_err(|error| {
            format!("Could not reach the game server at {path}. In development, use `nix run .#dev` so the backend starts with the Dioxus dev server. Details: {error:?}")
        })?;
    let response = response_value
        .dyn_into::<web_sys::Response>()
        .map_err(|error| format!("Invalid response for {path}: {error:?}"))?;

    parse_response(path, response).await
}

#[cfg(target_arch = "wasm32")]
async fn put_json<Request, Response>(path: &str, body: &Request) -> Result<Response, String>
where
    Request: Serialize,
    Response: DeserializeOwned,
{
    request_json("PUT", path, body).await
}

#[cfg(not(target_arch = "wasm32"))]
async fn put_json<Request, Response>(_path: &str, _body: &Request) -> Result<Response, String>
where
    Request: Serialize,
    Response: DeserializeOwned,
{
    Err("Browser API client is only available in the web build".to_string())
}

#[cfg(target_arch = "wasm32")]
async fn request_json<Request, Response>(
    method: &str,
    path: &str,
    body: &Request,
) -> Result<Response, String>
where
    Request: Serialize,
    Response: DeserializeOwned,
{
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;
    use wasm_bindgen_futures::JsFuture;

    let window = web_sys::window().ok_or_else(|| "Browser window is not available".to_string())?;
    let json = serde_json::to_string(body)
        .map_err(|error| format!("Could not serialize request: {error}"))?;
    let headers = web_sys::Headers::new()
        .map_err(|error| format!("Could not create request headers: {error:?}"))?;
    headers
        .set("Content-Type", "application/json")
        .map_err(|error| format!("Could not set request headers: {error:?}"))?;
    let options = web_sys::RequestInit::new();
    options.set_method(method);
    options.set_body(&JsValue::from_str(&json));
    options.set_headers(&headers);
    options.set_credentials(web_sys::RequestCredentials::Include);
    let response_value = JsFuture::from(window.fetch_with_str_and_init(path, &options))
        .await
        .map_err(|error| format!("Could not reach the game server at {path}: {error:?}"))?;
    let response = response_value
        .dyn_into::<web_sys::Response>()
        .map_err(|error| format!("Invalid response for {path}: {error:?}"))?;
    parse_response(path, response).await
}

#[cfg(target_arch = "wasm32")]
async fn parse_response<Response>(
    path: &str,
    response: web_sys::Response,
) -> Result<Response, String>
where
    Response: DeserializeOwned,
{
    use wasm_bindgen_futures::JsFuture;

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
            if response.status() == 405 && path.contains("/api/") {
                format!(
                    "The game server did not accept {path}. In development, use `nix run .#dev` instead of plain `dx serve`."
                )
            } else {
                format!("The game server returned HTTP {} for {path}", response.status())
            }
        }));
    }

    serde_json::from_str(&text).map_err(|error| format!("Could not parse {path}: {error}"))
}

#[cfg(not(target_arch = "wasm32"))]
async fn post_json<Request, Response>(_path: &str, _body: &Request) -> Result<Response, String>
where
    Request: Serialize,
    Response: DeserializeOwned,
{
    Err("Browser API client is only available in the web build".to_string())
}

fn error_message(text: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()?
        .get("error")?
        .as_str()
        .map(ToString::to_string)
}
