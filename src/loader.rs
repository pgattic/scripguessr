use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::api::{
    GuessRequest, GuessResponse, MetadataRequest, MetadataResponse, NewGameRequest, NewGameResponse,
};

pub async fn load_metadata(request: MetadataRequest) -> Result<MetadataResponse, String> {
    post_json(&api_path("/api/metadata"), &request).await
}

pub async fn create_game(request: NewGameRequest) -> Result<NewGameResponse, String> {
    post_json(&api_path("/api/games"), &request).await
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

    let response_value = JsFuture::from(window.fetch_with_str_and_init(path, &options))
        .await
        .map_err(|error| format!("Could not fetch {path}: {error:?}"))?;
    let response = response_value
        .dyn_into::<web_sys::Response>()
        .map_err(|error| format!("Invalid response for {path}: {error:?}"))?;

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
        return Err(error_message(&text)
            .unwrap_or_else(|| format!("Could not fetch {path}: HTTP {}", response.status())));
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
