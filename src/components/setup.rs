use dioxus::prelude::*;

use super::actions::request_new_game;
use super::review::StatsPanel;
use super::ui::{Callout, PanelHeader, Readout, ScopeEditor, Segmented, ToggleButton, use_game};
use crate::api::LEADERBOARD_ROUND_COUNTS;
use crate::routes::Route;
use crate::scriptures::GameMode;

#[component]
pub fn SetupPanel(load_error: Option<String>) -> Element {
    let mut game = use_game();
    let navigator = use_navigator();
    let mut scope_editor_open = use_signal(|| false);
    let snapshot = game.read().clone();
    let metadata_current = snapshot.metadata_current();
    let metadata_ready = snapshot.metadata_ready();
    let selection_label = snapshot.settings.selection_label();
    let verse_pool = format!(
        "{} of {} verses",
        snapshot.playable_verse_count(),
        snapshot.total_verse_count()
    );

    rsx! {
        div { class: "setup-layout",
            section { class: "panel setup-panel",
                PanelHeader { title: "New game", detail: "{selection_label}" }

                div { class: "scope-summary",
                    span { class: "muted", "Scope" }
                    strong { "{selection_label}" }
                    if metadata_current {
                        span { class: "muted", "{verse_pool} in play" }
                    } else {
                        span { class: "muted", "Updating verse pool" }
                    }
                }

                div { class: "setup-group",
                    span { class: "setup-label", "Presets" }
                    div { class: "preset-row",
                        for mode in GameMode::ALL {
                            ToggleButton {
                                active: mode.scope() == snapshot.settings.scope,
                                onclick: move |_| game.write().set_scope(mode.scope()),
                                "{mode.label()}"
                            }
                        }
                    }
                }

                div { class: "actions scope-actions",
                    button {
                        class: "button secondary",
                        onclick: move |_| scope_editor_open.toggle(),
                        if scope_editor_open() { "Close scope editor" } else { "Customize scope" }
                    }
                    button {
                        class: "button secondary",
                        onclick: move |_| {
                            navigator.push(Route::StudyIndex {});
                        },
                        "Study sets"
                    }
                    button {
                        class: "button secondary",
                        onclick: move |_| {
                            navigator.push(Route::Atlas {});
                        },
                        "Atlas"
                    }
                }

                if scope_editor_open() {
                    ScopeEditor {
                        class: "scope-editor",
                        scope: snapshot.settings.scope.clone(),
                        catalog: snapshot.catalog.as_ref().map(|catalog| catalog.metadata.clone()).unwrap_or_default(),
                        on_change: move |scope| game.write().set_scope(scope),
                    }
                }

                div { class: "setup-group",
                    span { class: "setup-label", "Rounds" }
                    div { class: "segmented",
                        for round_count in LEADERBOARD_ROUND_COUNTS {
                            ToggleButton {
                                active: snapshot.settings.round_count == round_count,
                                onclick: move |_| game.write().set_round_count(round_count),
                                "{round_count}"
                            }
                        }
                    }
                }

                div { class: "setup-group",
                    span { class: "setup-label", "Difficulty" }
                    Segmented {
                        value: snapshot.settings.difficulty,
                        onchange: move |difficulty| game.write().set_difficulty(difficulty),
                    }
                }

                div { class: "actions",
                    button {
                        class: "button",
                        disabled: !metadata_ready || snapshot.loading_game,
                        onclick: move |_| request_new_game(game, navigator),
                        if snapshot.loading_game { "Starting" } else { "Start" }
                    }
                }
                if let Some(error) = snapshot.error.clone() {
                    Callout { title: "Game unavailable", message: error }
                } else if metadata_current && !metadata_ready {
                    Callout {
                        title: "Scope unavailable",
                        message: "Choose at least one book with playable verses.",
                    }
                }
            }

            aside { class: "panel setup-side",
                Readout { label: "Verse pool",
                    if metadata_ready {
                        strong { "{verse_pool}" }
                    } else if metadata_current {
                        strong { "No playable verses" }
                    } else if load_error.is_some() {
                        strong { "Could not load game data" }
                    } else {
                        strong { "Loading game data" }
                    }
                }

                if let Some(error) = load_error {
                    Callout { title: "Game server unavailable", message: error }
                }

                StatsPanel {}
            }
        }
    }
}
