use dioxus::prelude::*;

use crate::api::{AuthRequest, ChangePasswordRequest};
use crate::game::Game;
use crate::loader::{
    change_password, load_account_data, load_leaderboard, login, logout, register,
};
use crate::routes::Route;
use crate::scriptures::{Difficulty, GameMode};

#[component]
pub fn LoginRoutePage() -> Element {
    rsx! { AuthPanel { registering: false } }
}

#[component]
pub fn RegisterRoutePage() -> Element {
    rsx! { AuthPanel { registering: true } }
}

#[component]
fn AuthPanel(registering: bool) -> Element {
    let mut game = use_context::<Signal<Game>>();
    let navigator = use_navigator();
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    rsx! {
        div { class: "setup-layout account-layout",
            section { class: "panel setup-panel account-panel",
                div { class: "picker-header",
                    h2 { if registering { "Create account" } else { "Sign in" } }
                }
                label { class: "setup-group",
                    span { class: "setup-label", "Username" }
                    input {
                        value: "{username}",
                        autocomplete: "username",
                        maxlength: 32,
                        oninput: move |event| username.set(event.value()),
                    }
                }
                label { class: "setup-group",
                    span { class: "setup-label", "Password" }
                    input {
                        r#type: "password",
                        value: "{password}",
                        autocomplete: if registering { "new-password" } else { "current-password" },
                        oninput: move |event| password.set(event.value()),
                    }
                }
                if let Some(message) = error() {
                    div { class: "callout warning", span { "{message}" } }
                }
                div { class: "actions",
                    button {
                        class: "button",
                        disabled: busy() || username().is_empty() || password().is_empty(),
                        onclick: move |_| {
                            busy.set(true);
                            error.set(None);
                            let request = AuthRequest {
                                username: username(),
                                password: password(),
                            };
                            spawn(async move {
                                let result = if registering { register(request).await } else { login(request).await };
                                match result {
                                    Ok(user) => match load_account_data().await {
                                        Ok(data) => {
                                            game.write().apply_user(Some(user));
                                            game.write().apply_account_data(data);
                                            navigator.replace(Route::Account {});
                                        }
                                        Err(message) => error.set(Some(message)),
                                    },
                                    Err(message) => error.set(Some(message)),
                                }
                                busy.set(false);
                            });
                        },
                        if busy() { "Working" } else if registering { "Create account" } else { "Sign in" }
                    }
                    button {
                        class: "button secondary",
                        onclick: move |_| {
                            if registering {
                                navigator.push(Route::Login {});
                            } else {
                                navigator.push(Route::Register {});
                            }
                        },
                        if registering { "I have an account" } else { "Create account" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn AccountRoutePage() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let navigator = use_navigator();
    let snapshot = game.read().clone();
    let mut current_password = use_signal(String::new);
    let mut new_password = use_signal(String::new);
    let mut message = use_signal(|| None::<String>);

    let Some(user) = snapshot.user else {
        return rsx! {
            section { class: "panel setup-panel",
                div { class: "ready",
                    strong { "Sign in to view your account" }
                    button { class: "button", onclick: move |_| { navigator.replace(Route::Login {}); }, "Sign in" }
                }
            }
        };
    };

    rsx! {
        div { class: "setup-layout account-layout",
            section { class: "panel setup-panel account-panel",
                div { class: "picker-header",
                    h2 { "{user.username}" }
                    span { class: "muted", "{snapshot.stats.games_played} games" }
                }
                div { class: "stat-grid",
                    div { span { class: "muted", "Games" } strong { "{snapshot.stats.games_played}" } }
                    div { span { class: "muted", "Rounds" } strong { "{snapshot.stats.rounds_played}" } }
                    div { span { class: "muted", "Average" } strong { "{snapshot.stats.average_percent().unwrap_or_default()}%" } }
                }
                div { class: "setup-group",
                    span { class: "setup-label", "Change password" }
                    input {
                        r#type: "password",
                        placeholder: "Current password",
                        value: "{current_password}",
                        oninput: move |event| current_password.set(event.value()),
                    }
                    input {
                        r#type: "password",
                        placeholder: "New password",
                        value: "{new_password}",
                        oninput: move |event| new_password.set(event.value()),
                    }
                }
                if let Some(text) = message() { p { class: "muted", "{text}" } }
                div { class: "actions",
                    button {
                        class: "button secondary",
                        disabled: current_password().is_empty() || new_password().is_empty(),
                        onclick: move |_| { spawn(async move {
                            let request = ChangePasswordRequest {
                                current_password: current_password(),
                                new_password: new_password(),
                            };
                            match change_password(request).await {
                                Ok(()) => {
                                    current_password.set(String::new());
                                    new_password.set(String::new());
                                    message.set(Some("Password changed".to_string()));
                                }
                                Err(error) => message.set(Some(error)),
                            }
                        }); },
                        "Change password"
                    }
                    button {
                        class: "button secondary",
                        onclick: move |_| { spawn(async move {
                            if logout().await.is_ok() {
                                game.write().apply_user(None);
                                navigator.replace(Route::Setup {});
                            }
                        }); },
                        "Sign out"
                    }
                }
            }
        }
    }
}

#[component]
pub fn LeaderboardsRoutePage() -> Element {
    let mut preset = use_signal(|| GameMode::BookOfMormon);
    let mut difficulty = use_signal(|| Difficulty::Normal);
    let mut rounds = use_signal(|| 5_usize);
    let resource =
        use_resource(
            move || async move { load_leaderboard(preset(), difficulty(), rounds()).await },
        );

    rsx! {
        section { class: "panel setup-panel leaderboard-panel",
            div { class: "picker-header", h2 { "Leaderboards" } }
            div { class: "leaderboard-filters",
                label { span { class: "setup-label", "Preset" }
                    select { value: "{preset():?}", onchange: move |event| {
                        if let Some(mode) = parse_mode(&event.value()) { preset.set(mode); }
                    },
                        for mode in GameMode::ALL { option { value: "{mode:?}", "{mode.label()}" } }
                    }
                }
                label { span { class: "setup-label", "Difficulty" }
                    select { value: "{difficulty():?}", onchange: move |event| {
                        if let Some(value) = parse_difficulty(&event.value()) { difficulty.set(value); }
                    },
                        for value in Difficulty::ALL { option { value: "{value:?}", "{value.label()}" } }
                    }
                }
                div { class: "segmented",
                    for count in [5_usize, 10] {
                        button { class: if rounds() == count { "segment active" } else { "segment" }, onclick: move |_| rounds.set(count), "{count}" }
                    }
                }
            }
            match resource.value().read().as_ref() {
                Some(Ok(board)) if board.entries.is_empty() => rsx! { p { class: "muted", "No completed games in this category yet." } },
                Some(Ok(board)) => rsx! {
                    div { class: "round-list leaderboard-list",
                        for entry in board.entries.iter() {
                            div { class: if entry.current_user { "round-row leaderboard-row current-user" } else { "round-row leaderboard-row" },
                                strong { "#{entry.rank}" }
                                span { "{entry.username}" }
                                span { "{entry.score} / {entry.possible_score}" }
                            }
                        }
                    }
                },
                Some(Err(error)) => rsx! { div { class: "callout warning", span { "{error}" } } },
                None => rsx! { p { class: "muted", "Loading standings" } },
            }
        }
    }
}

fn parse_mode(value: &str) -> Option<GameMode> {
    GameMode::ALL
        .into_iter()
        .find(|mode| format!("{mode:?}") == value)
}

fn parse_difficulty(value: &str) -> Option<Difficulty> {
    Difficulty::ALL
        .into_iter()
        .find(|difficulty| format!("{difficulty:?}") == value)
}
