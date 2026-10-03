use dioxus::prelude::*;

use super::ui::{Callout, ChoiceSelect, FormField, ToggleButton, use_game};
use crate::api::{
    AuthRequest, ChangePasswordRequest, LEADERBOARD_ROUND_COUNTS, LeaderboardEntry,
    LeaderboardQuery,
};
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
    let mut game = use_game();
    let navigator = use_navigator();
    let username = use_signal(String::new);
    let password = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let password_autocomplete = if registering {
        "new-password"
    } else {
        "current-password"
    };

    rsx! {
        div { class: "auth-layout",
            section { class: "panel auth-panel",
                div { class: "account-heading",
                    span { class: "section-kicker", if registering { "New account" } else { "Welcome back" } }
                    h2 { if registering { "Create account" } else { "Sign in" } }
                }
                div { class: "auth-fields",
                    FormField {
                        label: "Username",
                        value: username,
                        autocomplete: "username",
                        maxlength: 32,
                    }
                    FormField {
                        label: "Password",
                        value: password,
                        input_type: "password",
                        autocomplete: password_autocomplete,
                        minlength: 12,
                    }
                }
                if let Some(message) = error() {
                    Callout { message }
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
                                let result = async {
                                    let user = if registering {
                                        register(request).await?
                                    } else {
                                        login(request).await?
                                    };
                                    Ok::<_, String>((user, load_account_data().await?))
                                };
                                match result.await {
                                    Ok((user, data)) => {
                                        let mut game = game.write();
                                        game.account.apply_user(Some(user));
                                        game.account.apply_data(data);
                                        navigator.replace(Route::Account {});
                                    }
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
                            navigator.push(if registering { Route::Login {} } else { Route::Register {} });
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
    let mut game = use_game();
    let navigator = use_navigator();
    let account = game.read().account.clone();
    let mut current_password = use_signal(String::new);
    let mut new_password = use_signal(String::new);
    let mut message = use_signal(|| None::<String>);

    let Some(user) = account.user else {
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
    let review_count = account.review_items.len();

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
                    div { span { "Games" } strong { "{account.stats.games_played}" } }
                    div { span { "Rounds" } strong { "{account.stats.rounds_played}" } }
                    div { span { "Average" } strong { "{account.stats.average_percent().unwrap_or_default()}%" } }
                    div { span { "Marked" } strong { "{review_count}" } }
                }
                div { class: "account-links",
                    button { class: "account-link", onclick: move |_| { navigator.push(Route::Review {}); },
                        strong { "Review queue" }
                        span { "{review_count} passages" }
                    }
                    button { class: "account-link", onclick: move |_| { navigator.push(Route::StudyIndex {}); },
                        strong { "Custom study sets" }
                        span { "{account.study_sets.len()} sets" }
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
                    FormField {
                        label: "Current password",
                        value: current_password,
                        input_type: "password",
                        autocomplete: "current-password",
                    }
                    FormField {
                        label: "New password",
                        value: new_password,
                        input_type: "password",
                        autocomplete: "new-password",
                        minlength: 12,
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
                                game.write().account.apply_user(None);
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
    let mut rounds = use_signal(|| LEADERBOARD_ROUND_COUNTS[0]);
    let resource = use_resource(move || {
        load_leaderboard(LeaderboardQuery {
            preset: preset(),
            difficulty: difficulty(),
            rounds: rounds(),
        })
    });

    rsx! {
        section { class: "panel leaderboard-panel",
            div { class: "account-heading leaderboard-heading",
                span { class: "section-kicker", "Best scores" }
                h2 { "Leaderboards" }
            }
            div { class: "leaderboard-filters",
                label { class: "form-field", span { class: "setup-label", "Preset" }
                    ChoiceSelect { class: "form-control", value: preset(), onchange: move |mode| preset.set(mode) }
                }
                label { class: "form-field", span { class: "setup-label", "Difficulty" }
                    ChoiceSelect { class: "form-control", value: difficulty(), onchange: move |value| difficulty.set(value) }
                }
                label { class: "form-field leaderboard-round-filter",
                    span { class: "setup-label", "Rounds" }
                    div { class: "segmented",
                        for count in LEADERBOARD_ROUND_COUNTS {
                            ToggleButton { active: rounds() == count, onclick: move |_| rounds.set(count), "{count}" }
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
                    if let Some(entry) = board.current_user_entry.as_ref().filter(|_| !board.entries.iter().any(|entry| entry.current_user)) {
                        div { class: "leaderboard-personal",
                            span { class: "setup-label", "Your best" }
                            LeaderboardRow { entry: entry.clone() }
                        }
                    }
                },
                Some(Err(error)) => rsx! { Callout { message: error.clone() } },
                None => rsx! { div { class: "leaderboard-state", strong { "Loading standings" } } },
            }
        }
    }
}

#[component]
fn LeaderboardRow(entry: LeaderboardEntry) -> Element {
    rsx! {
        div { class: if entry.current_user { "leaderboard-row current-user" } else { "leaderboard-row" },
            strong { class: "leaderboard-rank", "#{entry.rank}" }
            span { class: "leaderboard-player", "{entry.username}" }
            div { class: "leaderboard-score",
                strong { "{entry.score} / {entry.possible_score}" }
                span { class: "muted", "{entry.percent()}%" }
            }
            span { class: "leaderboard-date muted", "{entry.completed_at}" }
        }
    }
}
