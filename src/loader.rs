use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;

use crate::scriptures::{Canon, Scriptures};

pub async fn load_scriptures(canon: Canon) -> Result<(Canon, Scriptures), String> {
    let data = fetch_text(canon.asset_url()).await?;
    let scriptures = Scriptures::from_flat_json_for_canon(canon, &data)
        .map_err(|error| format!("Could not parse {}: {error}", canon.label()))?;

    Ok((canon, scriptures))
}

#[cfg(target_arch = "wasm32")]
async fn fetch_text(url: Asset) -> Result<String, String> {
    let window = web_sys::window().ok_or_else(|| "Browser window is not available".to_string())?;
    let response_value = JsFuture::from(window.fetch_with_str(&url.to_string()))
        .await
        .map_err(|error| format!("Could not fetch {}: {error:?}", url))?;
    let response = response_value
        .dyn_into::<web_sys::Response>()
        .map_err(|error| format!("Invalid response for {}: {error:?}", url))?;

    if !response.ok() {
        return Err(format!(
            "Could not fetch {}: HTTP {}",
            url,
            response.status()
        ));
    }

    let text = JsFuture::from(
        response
            .text()
            .map_err(|error| format!("Could not read {}: {error:?}", url))?,
    )
    .await
    .map_err(|error| format!("Could not read {}: {error:?}", url))?;

    text.as_string()
        .ok_or_else(|| format!("Could not read {} as text", url))
}

#[cfg(not(target_arch = "wasm32"))]
async fn fetch_text(url: Asset) -> Result<String, String> {
    let path = url.resolve();

    std::fs::read_to_string(&path)
        .map_err(|error| format!("Could not read {}: {error}", path.display()))
}
