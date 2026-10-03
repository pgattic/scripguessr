use dioxus::prelude::*;

use crate::api::ChapterVerse;
use crate::components::dom::scroll_answer_verse_into_view;

#[component]
pub fn ChapterReader(
    title: String,
    #[props(default)] highlight: Vec<u16>,
    verses: Vec<ChapterVerse>,
    on_close: EventHandler<MouseEvent>,
) -> Element {
    use_effect(scroll_answer_verse_into_view);

    rsx! {
        div { class: "dialog-backdrop",
            div { class: "chapter-dialog", role: "dialog", aria_modal: "true",
                div { class: "dialog-header",
                    h2 { "{title}" }
                    button {
                        class: "clear-guess",
                        aria_label: "Close chapter reader",
                        title: "Close",
                        onclick: move |event| on_close.call(event),
                        "×"
                    }
                }
                div { class: "chapter-reader",
                    for verse in verses {
                        p {
                            class: if highlight.contains(&verse.verse) { "chapter-verse answer-verse" } else { "chapter-verse" },
                            sup { "{verse.verse}" }
                            "{verse.text}"
                        }
                    }
                }
            }
        }
    }
}
