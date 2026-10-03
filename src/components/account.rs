use dioxus::prelude::*;

use crate::api::{AuthRequest, ChangePasswordRequest, LeaderboardEntry};
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
        div { class: "auth-layout",
            section { class: "panel auth-panel",
                div { class: "account-heading",
                    span { class: "section-kicker", if registering { "New account" } else { "Welcome back" } }
                    h2 { if registering { "Create account" } else { "Sign in" } }
                }
                div { class: "auth-fields",
                label { class: "form-field",
                    span { class: "setup-label", "Username" }
                    input {
                        class: "form-control",
                        value: "{username}",
                        autocomplete: "username",
                        maxlength: 32,
                        oninput: move |event| username.set(event.value()),
                    }
                }
                label { class: "form-field",
                    span { class: "setup-label", "Password" }
                    input {
                        class: "form-control",
                        r#type: "password",
                        minlength: 12,
                        value: "{password}",
                        autocomplete: if registering { "new-password" } else { "current-password" },
                        oninput: move |event| password.set(event.value()),
                    }
                }
                }
                if let Some(message) = error() {
                    div { class: "callout warning", span { "{message}" } }
                }
                div { class: "actions auth-actions",
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
            section { class: "panel account-guest-panel",
                div { class: "account-empty-state",
                    span { class: "section-kicker", "Account" }
                    h2 { "Sign in to view your progress" }
                    div { class: "actions compact-actions",
                        button { class: "button", onclick: move |_| { navigator.replace(Route::Login {}); }, "Sign in" }
                        button { class: "button secondary", onclick: move |_| { navigator.replace(Route::Register {}); }, "Create account" }
                    }
                }
            }
        };
    };

    rsx! {
        div { class: "account-layout",
            section { class: "panel account-panel account-overview",
                div { class: "account-heading account-heading-row",
                    div {
                        span { class: "section-kicker", "Account" }
                        h2 { "{user.username}" }
                    }
                    span { class: "account-status", "Signed in" }
                }
                div { class: "account-stat-grid",
                    div { span { "Games" } strong { "{snapshot.stats.games_played}" } }
                    div { span { "Rounds" } strong { "{snapshot.stats.rounds_played}" } }
                    div { span { "Average" } strong { "{snapshot.stats.average_percent().unwrap_or_default()}%" } }
                    div { span { "Marked" } strong { "{snapshot.stats.review_items.len()}" } }
                }
                div { class: "account-links",
                    button { class: "account-link", onclick: move |_| { navigator.push(Route::Review {}); },
                        strong { "Review queue" }
                        span { "{snapshot.stats.review_items.len()} passages" }
                    }
                    button { class: "account-link", onclick: move |_| { navigator.push(Route::StudyIndex {}); },
                        strong { "Custom study sets" }
                        span { "{snapshot.custom_study_sets.sets.len()} sets" }
                    }
                    button { class: "account-link", onclick: move |_| { navigator.push(Route::Leaderboards {}); },
                        strong { "Leaderboards" }
                        span { "View standings" }
                    }
                }
            }
            aside { class: "panel account-panel security-panel",
                div { class: "account-heading",
                    span { class: "section-kicker", "Security" }
                    h2 { "Change password" }
                }
                div { class: "auth-fields",
                label { class: "form-field",
                    span { class: "setup-label", "Current password" }
                    input {
                        class: "form-control",
                        r#type: "password",
                        autocomplete: "current-password",
                        value: "{current_password}",
                        oninput: move |event| current_password.set(event.value()),
                    }
                }
                label { class: "form-field",
                    span { class: "setup-label", "New password" }
                    input {
                        class: "form-control",
                        r#type: "password",
                        minlength: 12,
                        autocomplete: "new-password",
                        value: "{new_password}",
                        oninput: move |event| new_password.set(event.value()),
                    }
                }
                }
                if let Some(text) = message() { div { class: "account-message", "{text}" } }
                div { class: "actions security-actions",
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
    let mut difficulty = use_signal(|| Difficulty::Easy);
    let mut rounds = use_signal(|| 5_usize);
    let resource =
        use_resource(
            move || async move { load_leaderboard(preset(), difficulty(), rounds()).await },
        );

    rsx! {
        section { class: "panel leaderboard-panel",
            div { class: "account-heading leaderboard-heading",
                span { class: "section-kicker", "Best scores" }
                h2 { "Leaderboards" }
            }
            div { class: "leaderboard-filters",
                label { class: "form-field", span { class: "setup-label", "Preset" }
                    select { class: "form-control", value: "{preset():?}", onchange: move |event| {
                        if let Some(mode) = parse_mode(&event.value()) { preset.set(mode); }
                    },
                        for mode in GameMode::ALL { option { value: "{mode:?}", "{mode.label()}" } }
                    }
                }
                label { class: "form-field", span { class: "setup-label", "Difficulty" }
                    select { class: "form-control", value: "{difficulty():?}", onchange: move |event| {
                        if let Some(value) = parse_difficulty(&event.value()) { difficulty.set(value); }
                    },
                        for value in Difficulty::ALL { option { value: "{value:?}", "{value.label()}" } }
                    }
                }
                label { class: "form-field leaderboard-round-filter",
                    span { class: "setup-label", "Rounds" }
                div { class: "segmented",
                    for count in [5_usize, 10] {
                        button { class: if rounds() == count { "segment active" } else { "segment" }, onclick: move |_| rounds.set(count), "{count}" }
                    }
                }
                }
            }
            match resource.value().read().as_ref() {
                Some(Ok(board)) if board.entries.is_empty() => rsx! {
                    div { class: "leaderboard-state",
                        strong { "No scores yet" }
                        span { class: "muted", "The first completed game will set the pace." }
                    }
                },
                Some(Ok(board)) => rsx! {
                    div { class: "leaderboard-table",
                        div { class: "leaderboard-table-header",
                            span { "Rank" }
                            span { "Player" }
                            span { "Score" }
                            span { "Date" }
                        }
                        for entry in board.entries.iter() {
                            LeaderboardRow { entry: entry.clone() }
                        }
                    }
                    if let Some(entry) = board.current_user_entry.as_ref().filter(|current| !board.entries.iter().any(|entry| entry.rank == current.rank)) {
                        div { class: "leaderboard-personal",
                            span { class: "setup-label", "Your best" }
                            LeaderboardRow { entry: entry.clone() }
                        }
                    }
                },
                Some(Err(error)) => rsx! { div { class: "callout warning", span { "{error}" } } },
                None => rsx! { div { class: "leaderboard-state", strong { "Loading standings" } } },
            }
        }
    }
}

#[component]
fn LeaderboardRow(entry: LeaderboardEntry) -> Element {
    let percent = if entry.possible_score == 0 {
        0
    } else {
        ((entry.score as f64 / entry.possible_score as f64) * 100.0).round() as u32
    };
    rsx! {
        div { class: if entry.current_user { "leaderboard-row current-user" } else { "leaderboard-row" },
            strong { class: "leaderboard-rank", "#{entry.rank}" }
            span { class: "leaderboard-player", "{entry.username}" }
            div { class: "leaderboard-score",
                strong { "{entry.score} / {entry.possible_score}" }
                span { class: "muted", "{percent}%" }
            }
            span { class: "leaderboard-date muted", "{entry.completed_at}" }
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
