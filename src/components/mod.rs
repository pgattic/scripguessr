mod account;
mod actions;
mod atlas;
mod dom;
mod pages;
mod play;
mod result;
mod review;
mod setup;
mod shell;
mod study_sets;
mod ui;

use dioxus::prelude::*;

pub use account::{AccountRoutePage, LeaderboardsRoutePage, LoginRoutePage, RegisterRoutePage};
pub use pages::{
    AtlasRoutePage, GameRoutePage, NotFoundRoutePage, ReviewRoutePage, SetupRoutePage,
    StudyIndexRoutePage, StudySetRoutePage,
};
pub use shell::AppShell;

use crate::game::Game;
use crate::loader::{current_user, load_account_data};
use crate::routes::Route;

#[component]
pub fn App() -> Element {
    let mut game = use_signal(Game::default);

    use_context_provider(|| game);
    let account_resource = use_resource(move || async move {
        let user = current_user().await?;
        let data = match user {
            Some(_) => Some(load_account_data().await?),
            None => None,
        };
        Ok::<_, String>((user, data))
    });
    use_effect(move || {
        let Some(Ok((user, data))) = account_resource.value().read().as_ref().cloned() else {
            return;
        };
        let mut game = game.write();
        game.account.apply_user(user);
        if let Some(data) = data {
            game.account.apply_data(data);
        }
    });

    rsx! {
        document::Stylesheet {
            href: asset!("/assets/main.css")
        }
        document::Meta {
            name: "viewport",
            content: "width=device-width, initial-scale=1"
        }
        Router::<Route> {}
    }
}
