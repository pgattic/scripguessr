mod components;
mod game;
mod loader;
mod scoring;
mod scriptures;
mod stats;

fn main() {
    dioxus::launch(components::App);
}
