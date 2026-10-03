use dioxus::prelude::*;

use super::atlas::AtlasPanel;
use super::play::{GuessPanel, VersePanel};
use super::review::ReviewPanel;
use super::setup::SetupPanel;
use super::study_sets::StudySetsPanel;
use super::ui::{Callout, Readout, use_game};
use crate::api::ScopeSummary;
use crate::game::Game;
use crate::loader::{load_game, load_metadata};
use crate::routes::Route;

#[component]
pub fn SetupRoutePage() -> Element {
    let mut game = use_game();
    use_effect(move || {
        if game.read().session().is_some() {
            game.write().change_settings();
        }
    });
    let mut metadata = use_resource(move || async move {
        let request = {
            let game = game.read();
            (!game.metadata_current()).then(|| game.metadata_request())
        };
        match request {
            Some(request) => load_metadata(request).await.map(Some),
            None => Ok(None),
        }
    });
    use_effect(move || {
        let Some(Ok(Some(summary))) = metadata.value().read().as_ref().cloned() else {
            return;
        };
        game.write().apply_metadata(summary);
        metadata.clear();
    });

    rsx! { SetupPanel { load_error: resource_error(&metadata) } }
}

#[component]
pub fn ReviewRoutePage() -> Element {
    rsx! { ReviewPanel {} }
}

#[component]
pub fn StudyIndexRoutePage() -> Element {
    let navigator = use_navigator();
    use_effect(move || {
        navigator.replace(Route::StudySet {
            set_id: "doctrinal-mastery-all".to_string(),
        });
    });
    rsx! {}
}

#[component]
pub fn StudySetRoutePage(set_id: String) -> Element {
    let catalog = use_study_catalog();
    rsx! {
        StudySetsPanel { selected_id: set_id, load_error: resource_error(&catalog) }
    }
}

#[component]
pub fn AtlasRoutePage() -> Element {
    let catalog = use_study_catalog();
    rsx! { AtlasPanel { load_error: resource_error(&catalog) } }
}

#[component]
pub fn GameRoutePage(game_id: String) -> Element {
    rsx! { GameRouteContent { key: "{game_id}", game_id } }
}

#[component]
fn GameRouteContent(game_id: String) -> Element {
    let mut game = use_game();
    let is_current = |game: &Game, id: &str| game.session().is_some_and(|session| session.id == id);
    let loaded = is_current(&game.read(), &game_id);
    let id = game_id.clone();
    let resource = use_resource(move || {
        let id = id.clone();
        async move {
            if loaded {
                Ok(None)
            } else {
                load_game(&id).await.map(Some)
            }
        }
    });
    use_effect(move || {
        let Some(Ok(Some(snapshot))) = resource.value().read().as_ref().cloned() else {
            return;
        };
        game.write().load_session(snapshot, None);
    });

    if let Some(error) = resource_error(&resource) {
        rsx! { GameUnavailable { message: error } }
    } else if is_current(&game.read(), &game_id) {
        rsx! {
            div { class: "layout",
                VersePanel {}
                GuessPanel {}
            }
        }
    } else {
        rsx! {
            section { class: "panel setup-panel",
                Readout { label: "Game session", strong { "Loading" } }
            }
        }
    }
}

#[component]
fn GameUnavailable(message: String) -> Element {
    let navigator = use_navigator();
    rsx! {
        section { class: "panel setup-panel",
            Callout { title: "Game unavailable", message }
            div { class: "actions",
                button {
                    class: "button",
                    onclick: move |_| {
                        navigator.replace(Route::Setup {});
                    },
                    "Start a new game"
                }
            }
        }
    }
}

#[component]
pub fn NotFoundRoutePage(segments: Vec<String>) -> Element {
    let navigator = use_navigator();
    use_effect(move || {
        navigator.replace(Route::Setup {});
    });
    rsx! {}
}

/// Loads every book in the standard works once, for pages that browse beyond the game scope.
fn use_study_catalog() -> Resource<Result<Option<ScopeSummary>, String>> {
    let mut game = use_game();
    let mut resource = use_resource(move || async move {
        if game.read().study_catalog.is_empty() {
            load_metadata(Game::study_catalog_request()).await.map(Some)
        } else {
            Ok(None)
        }
    });
    use_effect(move || {
        let Some(Ok(Some(summary))) = resource.value().read().as_ref().cloned() else {
            return;
        };
        game.write().apply_study_catalog(summary);
        resource.clear();
    });
    resource
}

fn resource_error<T>(resource: &Resource<Result<T, String>>) -> Option<String> {
    resource
        .value()
        .read()
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned())
}
