use dioxus::prelude::*;

use crate::api;
use crate::components::common::Prose;
use crate::state::use_app_state;

#[component]
pub fn NoteTile(note_id: i64) -> Element {
    let state = use_app_state();
    let note = use_resource(move || {
        let _ = state.notes_ver.read();
        async move { api::notes::get(note_id).await }
    });

    enum View {
        Loading,
        NotFound,
        Body(String),
    }
    let view = match note.read().as_ref() {
        None => View::Loading,
        Some(Err(_)) => View::NotFound,
        Some(Ok(n)) => View::Body(n.body.clone()),
    };

    match view {
        View::Loading => rsx! {
            div { class: "loading-text", "Loading…" }
        },
        View::NotFound => rsx! {
            div { class: "error-text", "Note not found" }
        },
        View::Body(body) => rsx! {
            Prose { text: body }
        },
    }
}
