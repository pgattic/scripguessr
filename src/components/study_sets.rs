use std::mem::discriminant;
use std::sync::Arc;

use dioxus::prelude::*;

use super::actions::{create_study_set, delete_study_set, request_study_game, update_study_set};
use super::ui::{
    Callout, ChoiceSelect, HomeButton, Readout, ScopeEditor, Segmented, ToggleButton, use_game,
};
use crate::game::catalog;
use crate::routes::Route;
use crate::scriptures::{Canon, GameScope};
use crate::study_sets::{
    MAX_VERSES_PER_PASSAGE, PromptPolicy, StudyGuessScope, StudyPassage, StudySet,
    built_in_study_sets,
};

#[component]
pub fn StudySetsPanel(selected_id: String, load_error: Option<String>) -> Element {
    let game = use_game();
    let navigator = use_navigator();
    let (custom_sets, signed_in, catalog_loaded) = {
        let game = game.read();
        (
            game.account.study_sets.clone(),
            game.account.user.is_some(),
            !game.study_catalog.is_empty(),
        )
    };
    let built_ins = built_in_study_sets().to_vec();
    let selected = built_ins
        .iter()
        .find(|set| set.id == selected_id)
        .map(|set| (set.clone(), false))
        .or_else(|| {
            custom_sets
                .iter()
                .find(|set| set.id == selected_id)
                .map(|set| (Arc::new(set.clone()), true))
        });

    rsx! {
        div { class: "study-layout",
            section { class: "panel study-list-panel",
                div { class: "picker-header",
                    h2 { "Study sets" }
                    button {
                        class: "button secondary compact-button",
                        disabled: !signed_in,
                        onclick: move |_| create_study_set(
                            game,
                            StudySet {
                                id: String::new(),
                                name: "Untitled set".to_string(),
                                passages: Vec::new(),
                                guess_scope: StudyGuessScope::FullCanons,
                                prompt_policy: PromptPolicy::Automatic,
                            },
                            move |set| {
                                navigator.push(Route::StudySet { set_id: set.id.clone() });
                            },
                        ),
                        if signed_in { "New set" } else { "Sign in to create" }
                    }
                }

                StudySetList {
                    title: "Built in",
                    sets: built_ins,
                    selected_id: selected_id.clone(),
                }
                if !custom_sets.is_empty() {
                    StudySetList {
                        title: "Custom",
                        class: "study-custom-label",
                        sets: custom_sets.into_iter().map(Arc::new).collect::<Vec<_>>(),
                        selected_id,
                    }
                }

                div { class: "actions",
                    HomeButton {}
                }
            }

            aside { class: "panel study-detail-panel",
                if let Some(error) = load_error {
                    Callout { title: "Study sets unavailable", message: error }
                } else if !catalog_loaded {
                    Readout { label: "Scripture catalog", strong { "Loading" } }
                } else if let Some((set, custom)) = selected {
                    StudySetDetail { set, custom }
                } else {
                    Readout { label: "Study set",
                        strong { "Not found" }
                        button {
                            class: "button secondary",
                            onclick: move |_| {
                                navigator.replace(Route::StudyIndex {});
                            },
                            "Browse study sets"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StudySetList(
    title: String,
    #[props(default)] class: &'static str,
    sets: Vec<Arc<StudySet>>,
    selected_id: String,
) -> Element {
    let navigator = use_navigator();
    rsx! {
        span { class: "setup-label {class}", "{title}" }
        div { class: "study-set-list",
            for set in sets {
                {
                    let id = set.id.clone();
                    rsx! {
                        button {
                            class: if selected_id == set.id { "study-set-row active" } else { "study-set-row" },
                            onclick: move |_| {
                                navigator.push(Route::StudySet { set_id: id.clone() });
                            },
                            strong { "{set.name}" }
                            span { class: "muted", "{set.passages.len()} passages" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StudySetDetail(set: ReadSignal<Arc<StudySet>>, custom: bool) -> Element {
    let game = use_game();
    let edit = move |change: &dyn Fn(&mut StudySet)| update_study_set(game, &set.read().id, change);
    let set = set();
    let navigator = use_navigator();
    let mut round_count = use_signal(|| 10_usize);
    let mut visible_passages = use_signal(|| 50_usize);
    let (study_catalog, loading_game, error) = {
        let game = game.read();
        (
            game.study_catalog.clone(),
            game.loading_game,
            game.error.clone(),
        )
    };
    let passage_count = set.passages.len();
    let guess_scope_covers_passages = set.guess_scope_covers_passages();
    let mut round_choices = vec![5_usize];
    if passage_count > 10 {
        round_choices.push(10);
    }
    if passage_count > 5 {
        round_choices.push(passage_count);
    }
    let answer_scopes = [
        ("Books in set", StudyGuessScope::BooksInSet),
        ("Full canons", StudyGuessScope::FullCanons),
        ("All works", StudyGuessScope::AllStandardWorks),
        (
            "Custom",
            StudyGuessScope::Custom(set.resolved_guess_scope()),
        ),
    ];

    rsx! {
        div { class: "study-detail",
            div { class: "picker-header",
                if custom {
                    input {
                        class: "study-name-input",
                        aria_label: "Study set name",
                        value: "{set.name}",
                        onchange: move |event| {
                            let name = event.value();
                            edit(&|set| set.name = name.clone());
                        },
                    }
                } else {
                    h2 { "{set.name}" }
                }
                span { class: "muted", "{passage_count} passages" }
            }

            if custom {
                PassageEditor { set_id: set.id.clone() }
            }

            div { class: "setup-group study-prompt-policy",
                span { class: "setup-label", "What should each round show?" }
                if custom {
                    Segmented {
                        class: "prompt-policy-options",
                        value: set.prompt_policy,
                        onchange: move |policy| edit(&|set| set.prompt_policy = policy),
                    }
                    p { class: "muted setting-description", "{set.prompt_policy.description()}" }
                } else {
                    Readout { label: "Rounds show", class: "compact-ready",
                        strong { "{set.prompt_policy.label()}" }
                    }
                }
            }

            div { class: "setup-group study-answer-scope",
                span { class: "setup-label", "Answer choices" }
                if custom {
                    div { class: "segmented answer-scope-options",
                        for (label, scope) in answer_scopes {
                            ToggleButton {
                                active: discriminant(&set.guess_scope) == discriminant(&scope),
                                onclick: move |_| edit(&|set| set.guess_scope = scope.clone()),
                                "{label}"
                            }
                        }
                    }
                    if let StudyGuessScope::Custom(scope) = set.guess_scope.clone() {
                        ScopeEditor {
                            class: "study-scope-editor",
                            scope,
                            catalog: study_catalog,
                            on_change: move |scope: GameScope| edit(&|set| set.guess_scope = StudyGuessScope::Custom(scope.clone())),
                        }
                    }
                } else {
                    Readout { label: "This set uses", class: "compact-ready",
                        strong { "{set.guess_scope.label()}" }
                    }
                }
                if !guess_scope_covers_passages {
                    Callout {
                        title: "Answer scope incomplete",
                        message: "Include every book represented in this set.",
                    }
                }
            }

            if set.passages.is_empty() {
                p { class: "muted", "Add at least one passage to practice this set." }
            } else {
                div { class: "study-passage-list",
                    for (index, passage) in set.passages.iter().take(visible_passages()).enumerate() {
                        div { class: "study-passage-row",
                            div { class: "study-passage-copy",
                                span { "{passage.label()}" }
                                small { class: "muted", "{set.prompt_policy.behavior_label(passage)}" }
                            }
                            if custom {
                                button {
                                    class: "text-button",
                                    onclick: move |_| edit(&|set| {
                                        if index < set.passages.len() {
                                            set.passages.remove(index);
                                        }
                                    }),
                                    "Remove"
                                }
                            }
                        }
                    }
                }
                if passage_count > visible_passages() {
                    div { class: "passage-list-footer",
                        span { class: "muted", "Showing {visible_passages()} of {passage_count}" }
                        button {
                            class: "button secondary compact-button",
                            onclick: move |_| visible_passages.set(
                                (visible_passages() + 50).min(passage_count)
                            ),
                            "Show more"
                        }
                    }
                }
            }

            if passage_count > 5 {
                div { class: "setup-group study-rounds",
                    span { class: "setup-label", "Rounds" }
                    div { class: "segmented",
                        for count in round_choices {
                            ToggleButton {
                                active: round_count().min(passage_count) == count,
                                onclick: move |_| round_count.set(count),
                                if count == passage_count { "All ({count})" } else { "{count}" }
                            }
                        }
                    }
                }
            }

            if let Some(error) = error {
                Callout { title: "Set unavailable", message: error }
            }

            div { class: "actions study-actions",
                if custom {
                    {
                        let id = set.id.clone();
                        rsx! {
                            button {
                                class: "button secondary",
                                onclick: move |_| {
                                    delete_study_set(game, id.clone());
                                    navigator.replace(Route::StudyIndex {});
                                },
                                "Delete set"
                            }
                        }
                    }
                }
                button {
                    class: "button",
                    disabled: set.passages.is_empty() || !guess_scope_covers_passages || loading_game,
                    onclick: move |_| request_study_game(
                        game,
                        navigator,
                        set.as_ref().clone(),
                        round_count().min(passage_count),
                    ),
                    if loading_game { "Starting" } else { "Practice set" }
                }
            }
        }
    }
}

#[component]
fn PassageEditor(set_id: String) -> Element {
    let game = use_game();
    let mut selected_canon = use_signal(|| Canon::BookOfMormon);
    let mut selected_book = use_signal(String::new);
    let mut chapter = use_signal(|| 1_u16);
    let mut start_verse = use_signal(|| 1_u16);
    let mut end_verse = use_signal(|| 1_u16);
    let canon = selected_canon();
    let books = catalog::books(&game.read().study_catalog, canon).to_vec();
    let book = books
        .iter()
        .find(|item| item.name == selected_book())
        .or_else(|| books.first())
        .cloned();
    let chapters = book
        .as_ref()
        .map(|book| book.chapters.clone())
        .unwrap_or_default();
    let max_verse = book
        .as_ref()
        .and_then(|book| book.verse_count(chapter()))
        .unwrap_or_default();
    let book_name = book.map(|book| book.name).unwrap_or_default();
    let valid = !book_name.is_empty()
        && chapters.contains(&chapter())
        && start_verse() > 0
        && end_verse() >= start_verse()
        && end_verse() <= max_verse
        && usize::from(end_verse() - start_verse()) < MAX_VERSES_PER_PASSAGE;
    let mut reset_verses = move || {
        start_verse.set(1);
        end_verse.set(1);
    };

    rsx! {
        div { class: "passage-editor",
            span { class: "setup-label", "Add passage" }
            div { class: "passage-fields",
                label {
                    span { "Canon" }
                    ChoiceSelect {
                        value: canon,
                        onchange: move |next| {
                            selected_canon.set(next);
                            selected_book.set(String::new());
                            chapter.set(1);
                            reset_verses();
                        },
                    }
                }
                label {
                    span { "Book" }
                    select {
                        value: "{book_name}",
                        onchange: move |event| {
                            selected_book.set(event.value());
                            chapter.set(1);
                            reset_verses();
                        },
                        for item in books.iter() {
                            option { value: "{item.name}", "{item.name}" }
                        }
                    }
                }
                label {
                    span { "Chapter" }
                    select {
                        value: "{chapter}",
                        onchange: move |event| {
                            if let Ok(value) = event.value().parse() {
                                chapter.set(value);
                                reset_verses();
                            }
                        },
                        for item in chapters {
                            option { value: "{item}", "{item}" }
                        }
                    }
                }
                VerseInput { label: "First verse", max: max_verse, value: start_verse }
                VerseInput { label: "Last verse", max: max_verse, value: end_verse }
                button {
                    class: "button",
                    disabled: !valid,
                    onclick: move |_| {
                        let passage = StudyPassage {
                            canon,
                            book: book_name.clone(),
                            chapter: chapter(),
                            verses: (start_verse()..=end_verse()).collect(),
                        };
                        update_study_set(game, &set_id, |set| {
                            if !set.passages.contains(&passage) {
                                set.passages.push(passage);
                            }
                        });
                    },
                    "Add"
                }
            }
        }
    }
}

#[component]
fn VerseInput(label: String, max: u16, value: Signal<u16>) -> Element {
    rsx! {
        label {
            span { "{label}" }
            input {
                r#type: "number",
                min: "1",
                max: "{max}",
                value: "{value}",
                oninput: move |event| {
                    if let Ok(next) = event.value().parse() {
                        value.set(next);
                    }
                },
            }
        }
    }
}
