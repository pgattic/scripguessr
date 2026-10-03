use dioxus::prelude::*;

use super::ui::use_game;
use crate::game::Game;
use crate::routes::Route;

#[component]
pub fn AppShell() -> Element {
    let game = use_game();
    let nav = use_navigator();
    let route = use_route::<Route>();
    let snapshot = game.read().clone();
    let subtitle = route_subtitle(&route, &snapshot);
    let pill = route_pill(&route, &snapshot);

    rsx! {
        main { class: "app",
            div { class: "shell",
                header { class: "topbar",
                    Link { class: "brand brand-link", to: Route::Setup {},
                        h1 { "ScripGuessr" }
                        span { "{subtitle}" }
                    }
                    if let Some(pill) = pill {
                        div { class: "pill", "{pill}" }
                    }
                }

                nav { class: "account-nav", aria_label: "Account navigation",
                    button {
                        class: "text-button",
                        onclick: move |_| { nav.push(Route::Leaderboards {}); },
                        "Leaderboards"
                    }
                    if let Some(user) = snapshot.account.user.as_ref() {
                        button {
                            class: "text-button",
                            onclick: move |_| { nav.push(Route::Account {}); },
                            "{user.username}"
                        }
                    } else if snapshot.account.loaded {
                        button {
                            class: "text-button",
                            onclick: move |_| { nav.push(Route::Login {}); },
                            "Sign in"
                        }
                    }
                }

                Outlet::<Route> {}
            }
        }
    }
}

fn route_subtitle(route: &Route, game: &Game) -> String {
    match route {
        Route::Game { .. } if game.session().is_some() => game.label(),
        Route::Game { .. } => "Loading game".to_string(),
        Route::Atlas {} => "Book of Mormon Atlas · people, narratives, and events".to_string(),
        Route::Review {} => "Marked passages".to_string(),
        Route::StudyIndex {} | Route::StudySet { .. } => {
            "Curated and custom study sets".to_string()
        }
        Route::Login {} => "Sign in".to_string(),
        Route::Register {} => "Create an account".to_string(),
        Route::Account {} => "Account and progress".to_string(),
        Route::Leaderboards {} => "Standard game leaderboards".to_string(),
        Route::Setup {} | Route::NotFound { .. } => setup_subtitle(game),
    }
}

fn setup_subtitle(game: &Game) -> String {
    let label = game.settings.selection_label();
    if game.settings.scope.canons.is_empty() {
        format!("{label} · choose a scope to start")
    } else if game.metadata_ready() {
        format!(
            "{label} · {} rounds · {} · {} of {} verses in play",
            game.settings.round_count,
            game.settings.difficulty.label(),
            game.playable_verse_count(),
            game.total_verse_count(),
        )
    } else {
        format!("{label} · loading game data")
    }
}

fn route_pill(route: &Route, game: &Game) -> Option<String> {
    match route {
        Route::Game { .. } => Some(format!(
            "Total {} / {}",
            game.total_score(),
            game.max_total_score()
        )),
        Route::Review {} => Some(format!("{} marked", game.account.review_items.len())),
        Route::StudyIndex {} | Route::StudySet { .. } => {
            Some(format!("{} custom", game.account.study_sets.len()))
        }
        Route::Atlas {} => Some("239 chapters".to_string()),
        _ => None,
    }
}
