mod chapter_reader;
mod scope_editor;

pub use chapter_reader::ChapterReader;
pub use scope_editor::ScopeEditor;

use dioxus::prelude::*;

use crate::game::Game;
use crate::routes::Route;
use crate::scriptures::{Canon, Difficulty, GameMode};
use crate::study_sets::PromptPolicy;

pub fn use_game() -> Signal<Game> {
    use_context()
}

pub trait Choice: Copy + PartialEq + 'static {
    const ALL: &'static [Self];

    fn label(self) -> &'static str;
}

impl Choice for Canon {
    const ALL: &'static [Self] = &Canon::ALL;

    fn label(self) -> &'static str {
        Canon::label(self)
    }
}

impl Choice for GameMode {
    const ALL: &'static [Self] = &GameMode::ALL;

    fn label(self) -> &'static str {
        GameMode::label(self)
    }
}

impl Choice for Difficulty {
    const ALL: &'static [Self] = &Difficulty::ALL;

    fn label(self) -> &'static str {
        Difficulty::label(self)
    }
}

impl Choice for PromptPolicy {
    const ALL: &'static [Self] = &PromptPolicy::ALL;

    fn label(self) -> &'static str {
        PromptPolicy::label(self)
    }
}

#[component]
pub fn ToggleButton(
    #[props(default = "segment")] kind: &'static str,
    active: bool,
    #[props(default)] disabled: bool,
    onclick: EventHandler<MouseEvent>,
    children: Element,
) -> Element {
    rsx! {
        button {
            class: if active { "{kind} active" } else { "{kind}" },
            disabled,
            onclick: move |event| onclick.call(event),
            {children}
        }
    }
}

#[component]
pub fn Segmented<T: Choice>(
    value: T,
    onchange: EventHandler<T>,
    #[props(default)] class: &'static str,
) -> Element {
    rsx! {
        div { class: "segmented {class}",
            for &option in T::ALL {
                ToggleButton {
                    active: option == value,
                    onclick: move |_| onchange.call(option),
                    "{option.label()}"
                }
            }
        }
    }
}

#[component]
pub fn ChoiceSelect<T: Choice>(
    value: T,
    onchange: EventHandler<T>,
    #[props(default)] class: &'static str,
) -> Element {
    let selected = T::ALL
        .iter()
        .position(|option| *option == value)
        .unwrap_or(0);
    rsx! {
        select {
            class,
            value: "{selected}",
            onchange: move |event| {
                if let Some(option) = event.value().parse::<usize>().ok().and_then(|index| T::ALL.get(index)) {
                    onchange.call(*option);
                }
            },
            for (index, option) in T::ALL.iter().enumerate() {
                option { value: "{index}", "{option.label()}" }
            }
        }
    }
}

#[component]
pub fn Callout(title: Option<String>, message: String) -> Element {
    rsx! {
        div { class: "callout warning",
            if let Some(title) = title {
                strong { "{title}" }
            }
            span { "{message}" }
        }
    }
}

/// A muted label above a value, laid out by the `ready` class.
#[component]
pub fn Readout(label: String, #[props(default)] class: &'static str, children: Element) -> Element {
    rsx! {
        div { class: "ready {class}",
            span { class: "muted", "{label}" }
            {children}
        }
    }
}

#[component]
pub fn PanelHeader(title: String, detail: String) -> Element {
    rsx! {
        div { class: "picker-header",
            h2 { "{title}" }
            span { class: "muted", "{detail}" }
        }
    }
}

#[component]
pub fn HomeButton() -> Element {
    let mut game = use_game();
    let navigator = use_navigator();
    rsx! {
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

#[component]
pub fn FormField(
    label: String,
    value: Signal<String>,
    #[props(default = "text")] input_type: &'static str,
    autocomplete: &'static str,
    minlength: Option<u32>,
    maxlength: Option<u32>,
) -> Element {
    rsx! {
        label { class: "form-field",
            span { class: "setup-label", "{label}" }
            input {
                class: "form-control",
                r#type: input_type,
                autocomplete,
                minlength,
                maxlength,
                value: "{value}",
                oninput: move |event| value.set(event.value()),
            }
        }
    }
}
