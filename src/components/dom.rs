use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::ScrollLogicalPosition;

pub fn scroll_round_into_view() {
    scroll_into_view(".verse-panel", ScrollLogicalPosition::Start);
}

pub fn scroll_result_into_view() {
    scroll_into_view(".result", ScrollLogicalPosition::Start);
}

pub fn scroll_answer_verse_into_view() {
    scroll_into_view(
        ".chapter-dialog .answer-verse",
        ScrollLogicalPosition::Center,
    );
}

/// Waits one tick so elements from the current render exist.
fn scroll_into_view(selector: &'static str, block: ScrollLogicalPosition) {
    let Some(window) = web_sys::window() else {
        return;
    };

    let callback = Closure::once(move || {
        let Some(element) = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.query_selector(selector).ok().flatten())
        else {
            return;
        };

        let options = web_sys::ScrollIntoViewOptions::new();
        options.set_behavior(web_sys::ScrollBehavior::Smooth);
        options.set_block(block);
        element.scroll_into_view_with_scroll_into_view_options(&options);
    });

    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        callback.as_ref().unchecked_ref(),
        0,
    );
    callback.forget();
}
