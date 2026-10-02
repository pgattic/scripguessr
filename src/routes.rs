use dioxus::prelude::*;

use crate::components::{
    AccountRoutePage, AppShell, AtlasRoutePage, GameRoutePage, LeaderboardsRoutePage,
    LoginRoutePage, NotFoundRoutePage, RegisterRoutePage, ReviewRoutePage, SetupRoutePage,
    StudyIndexRoutePage, StudySetRoutePage,
};

#[derive(Clone, Debug, PartialEq, Routable)]
#[rustfmt::skip]
pub enum Route {
    #[layout(AppShell)]
        #[route("/", SetupRoutePage)]
        Setup {},
        #[route("/review", ReviewRoutePage)]
        Review {},
        #[route("/study", StudyIndexRoutePage)]
        StudyIndex {},
        #[route("/study/:set_id", StudySetRoutePage)]
        StudySet { set_id: String },
        #[route("/atlas", AtlasRoutePage)]
        Atlas {},
        #[route("/game/v1/:game_id", GameRoutePage)]
        Game { game_id: String },
        #[route("/login", LoginRoutePage)]
        Login {},
        #[route("/register", RegisterRoutePage)]
        Register {},
        #[route("/account", AccountRoutePage)]
        Account {},
        #[route("/leaderboards", LeaderboardsRoutePage)]
        Leaderboards {},
        #[route("/:..segments", NotFoundRoutePage)]
        NotFound { segments: Vec<String> },
}
