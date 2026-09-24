use dioxus::prelude::*;

use crate::components::{
    AppShell, AtlasRoutePage, NotFoundRoutePage, ReviewRoutePage, SetupRoutePage,
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
        #[route("/:..segments", NotFoundRoutePage)]
        NotFound { segments: Vec<String> },
}
