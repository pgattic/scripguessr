mod api;
#[cfg(target_arch = "wasm32")]
mod components;
#[cfg(target_arch = "wasm32")]
mod game;
#[cfg(target_arch = "wasm32")]
mod loader;
mod scoring;
mod scriptures;
#[cfg(any(test, target_arch = "wasm32"))]
mod stats;
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
mod study_sets;

#[cfg(target_arch = "wasm32")]
fn main() {
    dioxus::launch(components::App);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    server::serve().await.expect("server failed");
}

#[cfg(not(target_arch = "wasm32"))]
mod server;
