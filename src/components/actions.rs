use std::future::Future;

use dioxus::prelude::*;
use dioxus::router::Navigator;

use super::dom::{scroll_result_into_view, scroll_round_into_view};
use crate::api::NewGameRequest;
use crate::game::{Game, ReviewToggle};
use crate::loader;
use crate::routes::Route;
use crate::study_sets::{StudyPassage, StudySet};

/// Runs a server request in the background and shows any failure as the page error.
pub fn spawn_request<T: 'static>(
    mut game: Signal<Game>,
    request: impl Future<Output = Result<T, String>> + 'static,
    on_success: impl FnOnce(T) + 'static,
) {
    spawn(async move {
        match request.await {
            Ok(value) => on_success(value),
            Err(error) => game.write().fail_request(error),
        }
    });
}

pub fn request_new_game(game: Signal<Game>, navigator: Navigator) {
    let request = game.read().new_game_request();
    launch_game(game, navigator, request, None);
}

pub fn request_study_game(
    game: Signal<Game>,
    navigator: Navigator,
    set: StudySet,
    round_count: usize,
) {
    let Some(request) = game.read().study_game_request(&set, round_count) else {
        return;
    };
    launch_game(game, navigator, request, Some(set.name));
}

fn launch_game(
    mut game: Signal<Game>,
    navigator: Navigator,
    request: NewGameRequest,
    label: Option<String>,
) {
    game.write().begin_starting_game();
    spawn_request(game, loader::create_game(request), move |snapshot| {
        let game_id = snapshot.game_id.clone();
        game.write().load_session(snapshot, label);
        navigator.push(Route::Game { game_id });
        scroll_round_into_view();
    });
}

pub fn request_guess_submission(mut game: Signal<Game>) {
    let session_id = game.read().session().map(|session| session.id.clone());
    let request = game.read().guess_request();
    let (Some(game_id), Some(request)) = (session_id, request) else {
        return;
    };

    game.write().begin_submitting_guess();
    spawn_request(
        game,
        async move { loader::submit_guess(&game_id, request).await },
        move |guess| {
            game.write().apply_guess(guess);
            scroll_result_into_view();
        },
    );
}

pub fn request_round_advance(mut game: Signal<Game>) {
    let Some(game_id) = game.read().session().map(|session| session.id.clone()) else {
        return;
    };
    spawn_request(
        game,
        async move { loader::advance_game(&game_id).await },
        move |response| {
            let finished = response.finished;
            game.write().apply_advance(response);
            if !finished {
                scroll_round_into_view();
            } else if game.read().account.user.is_some() {
                spawn_request(game, loader::load_account_data(), move |data| {
                    game.write().account.apply_data(data);
                });
            }
        },
    );
}

pub fn toggle_review(mut game: Signal<Game>) {
    let Some(item) = game.read().current_review_item() else {
        return;
    };
    let toggle = game.write().account.toggle_review(item);
    match toggle {
        ReviewToggle::Added(item) => spawn_request(game, loader::save_review_item(item), drop),
        ReviewToggle::Removed(passage) => {
            spawn_request(game, loader::remove_review_item(passage), drop)
        }
    }
}

pub fn unmark_review(mut game: Signal<Game>, passage: StudyPassage) {
    game.write().account.remove_review(&passage);
    spawn_request(game, loader::remove_review_item(passage), drop);
}

pub fn create_study_set(
    mut game: Signal<Game>,
    set: StudySet,
    on_saved: impl FnOnce(&StudySet) + 'static,
) {
    spawn_request(game, loader::save_study_set(set), move |set| {
        game.write().account.upsert_study_set(set.clone());
        on_saved(&set);
    });
}

/// Edits a custom set locally, then saves it.
pub fn update_study_set(mut game: Signal<Game>, id: &str, edit: impl FnOnce(&mut StudySet)) {
    let Some(set) = game.write().account.edit_study_set(id, edit) else {
        return;
    };
    spawn_request(game, loader::save_study_set(set), move |set| {
        game.write().account.upsert_study_set(set);
    });
}

pub fn delete_study_set(mut game: Signal<Game>, id: String) {
    game.write().account.delete_study_set(&id);
    spawn_request(game, loader::remove_study_set(id), drop);
}
