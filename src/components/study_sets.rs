use std::sync::Arc;

use dioxus::prelude::*;

use super::request_study_game;
use crate::game::Game;
use crate::routes::Route;
use crate::scriptures::{BookScope, Canon};
use crate::study_sets::{
    PromptPolicy, StudyGuessScope, StudyPassage, StudySet, built_in_study_sets,
};

#[component]
pub(super) fn StudySetsPanel(
    game: Signal<Game>,
    selected_id: String,
    load_error: Option<String>,
) -> Element {
    let navigator = use_navigator();
    let snapshot = game.read().clone();
    let built_ins = built_in_study_sets();
    let custom_sets = snapshot.custom_study_sets.sets.clone();
    let selected = built_ins
        .iter()
        .find(|set| set.id == selected_id)
        .cloned()
        .map(|set| (set, false))
        .or_else(|| {
            custom_sets
                .iter()
                .find(|set| set.id == selected_id)
                .cloned()
                .map(|set| (Arc::new(set), true))
        });

    rsx! {
        div { class: "study-layout",
            section { class: "panel study-list-panel",
                div { class: "picker-header",
                    h2 { "Study sets" }
                    button {
                        class: "button secondary compact-button",
                        onclick: move |_| {
                            let id = game.write().create_study_set();
                            navigator.push(Route::StudySet { set_id: id });
                        },
                        "New set"
                    }
                }

                span { class: "setup-label", "Built in" }
                div { class: "study-set-list",
                    for set in built_ins.iter() {
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

                if !custom_sets.is_empty() {
                    span { class: "setup-label study-custom-label", "Custom" }
                    div { class: "study-set-list",
                        for set in custom_sets.iter() {
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

                div { class: "actions",
                    button {
                        class: "button secondary",
                        onclick: move |_| {
                            game.write().change_settings();
                            navigator.push(Route::Setup {});
                        },
                        "Home"
                    }
                }
            }

            aside { class: "panel study-detail-panel",
                if let Some(error) = load_error {
                    div { class: "callout warning",
                        strong { "Study sets unavailable" }
                        span { "{error}" }
                    }
                } else if snapshot.study_metadata.is_empty() {
                    div { class: "ready",
                        span { class: "muted", "Scripture catalog" }
                        strong { "Loading" }
                    }
                } else if let Some((set, custom)) = selected {
                    StudySetDetail {
                        game,
                        set,
                        custom,
                    }
                } else {
                    div { class: "ready",
                        span { class: "muted", "Study set" }
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
fn StudySetDetail(game: Signal<Game>, set: Arc<StudySet>, custom: bool) -> Element {
    let navigator = use_navigator();
    let mut round_count = use_signal(|| 10_usize);
    let mut visible_passages = use_signal(|| 50_usize);
    let snapshot = game.read().clone();
    let passage_count = set.passages.len();
    let set_id = set.id.clone();
    let practice_set = set.as_ref().clone();
    let guess_scope_covers_passages = set.guess_scope_covers_passages();
    let custom_scope_start = set.resolved_guess_scope();
    let mut round_choices = vec![5_usize];
    if set.passages.len() > 10 {
        round_choices.push(10);
    }
    if set.passages.len() > 5 {
        round_choices.push(set.passages.len());
    }

    rsx! {
        div { class: "study-detail",
            div { class: "picker-header",
                if custom {
                    input {
                        class: "study-name-input",
                        aria_label: "Study set name",
                        value: "{set.name}",
                        oninput: move |event| game.write().rename_study_set(&set_id, event.value()),
                    }
                } else {
                    h2 { "{set.name}" }
                }
                span { class: "muted", "{set.passages.len()} passages" }
            }

            if custom {
                PassageEditor { game, set_id: set.id.clone() }
            }

            div { class: "setup-group study-prompt-policy",
                span { class: "setup-label", "What should each round show?" }
                if custom {
                    div { class: "segmented prompt-policy-options",
                        for policy in PromptPolicy::ALL {
                            button {
                                class: if set.prompt_policy == policy { "segment active" } else { "segment" },
                                onclick: {
                                    let id = set.id.clone();
                                    move |_| game.write().set_study_prompt_policy(&id, policy)
                                },
                                "{policy.label()}"
                            }
                        }
                    }
                    p { class: "muted setting-description", "{set.prompt_policy.description()}" }
                } else {
                    div { class: "ready compact-ready",
                        span { class: "muted", "Rounds show" }
                        strong { "{set.prompt_policy.label()}" }
                    }
                }
            }

            div { class: "setup-group study-answer-scope",
                span { class: "setup-label", "Answer choices" }
                if custom {
                    div { class: "segmented answer-scope-options",
                        button {
                            class: if matches!(set.guess_scope, StudyGuessScope::BooksInSet) { "segment active" } else { "segment" },
                            onclick: {
                                let id = set.id.clone();
                                move |_| game.write().set_study_guess_scope(&id, StudyGuessScope::BooksInSet)
                            },
                            "Books in set"
                        }
                        button {
                            class: if matches!(set.guess_scope, StudyGuessScope::FullCanons) { "segment active" } else { "segment" },
                            onclick: {
                                let id = set.id.clone();
                                move |_| game.write().set_study_guess_scope(&id, StudyGuessScope::FullCanons)
                            },
                            "Full canons"
                        }
                        button {
                            class: if matches!(set.guess_scope, StudyGuessScope::AllStandardWorks) { "segment active" } else { "segment" },
                            onclick: {
                                let id = set.id.clone();
                                move |_| game.write().set_study_guess_scope(&id, StudyGuessScope::AllStandardWorks)
                            },
                            "All works"
                        }
                        button {
                            class: if matches!(set.guess_scope, StudyGuessScope::Custom(_)) { "segment active" } else { "segment" },
                            onclick: {
                                let id = set.id.clone();
                                move |_| game.write().set_study_guess_scope(
                                    &id,
                                    StudyGuessScope::Custom(custom_scope_start.clone()),
                                )
                            },
                            "Custom"
                        }
                    }
                    if let StudyGuessScope::Custom(scope) = set.guess_scope.clone() {
                        StudyGuessScopeEditor { game, set_id: set.id.clone(), scope }
                    }
                } else {
                    div { class: "ready compact-ready",
                        span { class: "muted", "This set uses" }
                        strong { "{set.guess_scope.label()}" }
                    }
                }
                if !guess_scope_covers_passages {
                    div { class: "callout warning",
                        strong { "Answer scope incomplete" }
                        span { "Include every book represented in this set." }
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
                                {
                                    let remove_id = set.id.clone();
                                    rsx! {
                                        button {
                                            class: "text-button",
                                            onclick: move |_| game.write().remove_study_passage(&remove_id, index),
                                            "Remove"
                                        }
                                    }
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

            if set.passages.len() > 5 {
                div { class: "setup-group study-rounds",
                    span { class: "setup-label", "Rounds" }
                    div { class: "segmented",
                        for count in round_choices {
                            button {
                                class: if round_count().min(set.passages.len()) == count { "segment active" } else { "segment" },
                                onclick: move |_| round_count.set(count),
                                if count == set.passages.len() { "All ({count})" } else { "{count}" }
                            }
                        }
                    }
                }
            }

            if let Some(error) = snapshot.error.as_ref() {
                div { class: "callout warning",
                    strong { "Set unavailable" }
                    span { "{error}" }
                }
            }

            div { class: "actions study-actions",
                if custom {
                    {
                        let delete_id = set.id.clone();
                        rsx! {
                            button {
                                class: "button secondary",
                                onclick: move |_| {
                                    game.write().delete_study_set(&delete_id);
                                    navigator.replace(Route::StudyIndex {});
                                },
                                "Delete set"
                            }
                        }
                    }
                }
                button {
                    class: "button",
                    disabled: set.passages.is_empty() || !guess_scope_covers_passages || snapshot.loading_game,
                    onclick: move |_| request_study_game(
                        game,
                        navigator,
                        practice_set.clone(),
                        round_count().min(practice_set.passages.len()),
                    ),
                    if snapshot.loading_game { "Starting" } else { "Practice set" }
                }
            }
        }
    }
}

#[component]
fn StudyGuessScopeEditor(
    game: Signal<Game>,
    set_id: String,
    scope: crate::scriptures::GameScope,
) -> Element {
    let metadata = game.read().study_metadata.clone();

    rsx! {
        div { class: "study-scope-editor",
            for canon in Canon::ALL {
                {
                    let selected = scope.contains_canon(canon);
                    let canon_scope = scope.canon_scope(canon).cloned();
                    let books = metadata
                        .iter()
                        .find(|item| item.canon == canon)
                        .map(|item| item.books.clone())
                        .unwrap_or_default();
                    rsx! {
                        div { class: if selected { "scope-canon selected" } else { "scope-canon" },
                            div { class: "scope-canon-header",
                                button {
                                    class: if selected { "choice active" } else { "choice" },
                                    onclick: {
                                        let id = set_id.clone();
                                        let mut next = scope.clone();
                                        move |_| {
                                            if selected {
                                                next.canons.retain(|item| item.canon != canon);
                                            } else {
                                                next.canons.push(crate::scriptures::CanonScope {
                                                    canon,
                                                    books: BookScope::All,
                                                });
                                                next.canons.sort_by_key(|item| Canon::ALL.iter().position(|candidate| *candidate == item.canon));
                                            }
                                            game.write().set_study_guess_scope(&id, StudyGuessScope::Custom(next.clone()));
                                        }
                                    },
                                    "{canon.label()}"
                                }
                            }
                            if let Some(canon_scope) = canon_scope {
                                div { class: "segmented scope-mode",
                                    button {
                                        class: if canon_scope.books == BookScope::All { "segment active" } else { "segment" },
                                        onclick: {
                                            let id = set_id.clone();
                                            let mut next = scope.clone();
                                            move |_| {
                                                if let Some(item) = next.canons.iter_mut().find(|item| item.canon == canon) {
                                                    item.books = BookScope::All;
                                                }
                                                game.write().set_study_guess_scope(&id, StudyGuessScope::Custom(next.clone()));
                                            }
                                        },
                                        "All books"
                                    }
                                    button {
                                        class: if matches!(canon_scope.books, BookScope::Selected(_)) { "segment active" } else { "segment" },
                                        onclick: {
                                            let id = set_id.clone();
                                            let mut next = scope.clone();
                                            move |_| {
                                                if let Some(item) = next.canons.iter_mut().find(|item| item.canon == canon) {
                                                    item.books = BookScope::Selected(Vec::new());
                                                }
                                                game.write().set_study_guess_scope(&id, StudyGuessScope::Custom(next.clone()));
                                            }
                                        },
                                        "Choose books"
                                    }
                                }
                                if let BookScope::Selected(ref selected_books) = canon_scope.books {
                                    div { class: "book-select-grid study-book-grid",
                                        for book in books {
                                            {
                                                let active = selected_books.contains(&book.name);
                                                let book_name = book.name.clone();
                                                let selected_books = selected_books.clone();
                                                rsx! {
                                                    button {
                                                        class: if active { "choice active" } else { "choice" },
                                                        onclick: {
                                                            let id = set_id.clone();
                                                            let mut next = scope.clone();
                                                            move |_| {
                                                                let mut names = selected_books.clone();
                                                                if active {
                                                                    names.retain(|name| name != &book_name);
                                                                } else {
                                                                    names.push(book_name.clone());
                                                                }
                                                                if let Some(item) = next.canons.iter_mut().find(|item| item.canon == canon) {
                                                                    item.books = BookScope::Selected(names.clone());
                                                                }
                                                                game.write().set_study_guess_scope(&id, StudyGuessScope::Custom(next.clone()));
                                                            }
                                                        },
                                                        "{book.name}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn PassageEditor(game: Signal<Game>, set_id: String) -> Element {
    let snapshot = game.read().clone();
    let mut selected_canon = use_signal(|| Canon::BookOfMormon);
    let mut selected_book = use_signal(String::new);
    let mut chapter = use_signal(|| 1_u16);
    let mut start_verse = use_signal(|| 1_u16);
    let mut end_verse = use_signal(|| 1_u16);
    let canon = selected_canon();
    let books = snapshot
        .study_metadata
        .iter()
        .find(|metadata| metadata.canon == canon)
        .map(|metadata| metadata.books.clone())
        .unwrap_or_default();
    let book = if books.iter().any(|item| item.name == selected_book()) {
        selected_book()
    } else {
        books
            .first()
            .map(|item| item.name.clone())
            .unwrap_or_default()
    };
    let chapters = books
        .iter()
        .find(|item| item.name == book)
        .map(|item| item.chapters.clone())
        .unwrap_or_default();
    let max_verse = books
        .iter()
        .find(|item| item.name == book)
        .and_then(|item| item.verse_count(chapter()))
        .unwrap_or_default();
    let valid = !book.is_empty()
        && chapters.contains(&chapter())
        && start_verse() > 0
        && end_verse() >= start_verse()
        && end_verse() <= max_verse
        && end_verse() - start_verse() < 200;

    rsx! {
        div { class: "passage-editor",
            span { class: "setup-label", "Add passage" }
            div { class: "passage-fields",
                label {
                    span { "Canon" }
                    select {
                        value: "{canon.label()}",
                        onchange: move |event| {
                            if let Some(next) = Canon::ALL.into_iter().find(|item| item.label() == event.value()) {
                                selected_canon.set(next);
                                selected_book.set(String::new());
                                chapter.set(1);
                                start_verse.set(1);
                                end_verse.set(1);
                            }
                        },
                        for item in Canon::ALL {
                            option { value: "{item.label()}", "{item.label()}" }
                        }
                    }
                }
                label {
                    span { "Book" }
                    select {
                        value: "{book}",
                        onchange: move |event| {
                            selected_book.set(event.value());
                            chapter.set(1);
                            start_verse.set(1);
                            end_verse.set(1);
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
                                start_verse.set(1);
                                end_verse.set(1);
                            }
                        },
                        for item in chapters {
                            option { value: "{item}", "{item}" }
                        }
                    }
                }
                label {
                    span { "First verse" }
                    input {
                        r#type: "number",
                        min: "1",
                        max: "{max_verse}",
                        value: "{start_verse}",
                        oninput: move |event| {
                            if let Ok(value) = event.value().parse() {
                                start_verse.set(value);
                            }
                        },
                    }
                }
                label {
                    span { "Last verse" }
                    input {
                        r#type: "number",
                        min: "1",
                        max: "{max_verse}",
                        value: "{end_verse}",
                        oninput: move |event| {
                            if let Ok(value) = event.value().parse() {
                                end_verse.set(value);
                            }
                        },
                    }
                }
                button {
                    class: "button",
                    disabled: !valid,
                    onclick: move |_| {
                        game.write().add_study_passage(
                            &set_id,
                            StudyPassage {
                                canon,
                                book: book.clone(),
                                chapter: chapter(),
                                verses: (start_verse()..=end_verse()).collect(),
                            },
                        );
                    },
                    "Add"
                }
            }
        }
    }
}
