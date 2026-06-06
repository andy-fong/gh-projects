use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{LdExternalLink, LdPlus, LdSquarePen, LdTrash2};
use dioxus_free_icons::Icon;

use crate::api;
use crate::components::common::Prose;
use crate::components::dialogs::note_dialog::NoteDialog;
use crate::field_extractors::format_date_short;
use crate::state::use_app_state;
use crate::types::{CreateNoteInput, Note};

fn github_url(note: &Note) -> Option<String> {
    let repo = note.repo.as_ref()?;
    let num = note.ref_number?;
    let kind = if note.ref_type.as_deref() == Some("pr") {
        "pull"
    } else {
        "issues"
    };
    Some(format!("https://github.com/{repo}/{kind}/{num}"))
}

#[component]
pub fn NotesPage() -> Element {
    let state = use_app_state();

    let notes = use_resource(move || {
        let _ = state.notes_ver.read();
        async move { api::notes::list(api::notes::Filter::default()).await }
    });

    let mut selected = use_signal(|| None::<Note>);
    let mut show_create = use_signal(|| false);
    let mut editing = use_signal(|| None::<Note>);

    let on_create = move |input: CreateNoteInput| {
        spawn(async move {
            if api::notes::create(&input).await.is_ok() {
                state.invalidate_notes();
            }
        });
        show_create.set(false);
    };

    let on_update = move |input: CreateNoteInput| {
        let Some(note) = editing.read().clone() else {
            return;
        };
        spawn(async move {
            if api::notes::update(note.id, &input).await.is_ok() {
                state.invalidate_notes();
            }
        });
        editing.set(None);
        selected.set(None);
    };

    let delete_note = move |note: Note| {
        let confirmed = web_sys::window()
            .and_then(|w| w.confirm_with_message(&format!("Delete \"{}\"?", note.title)).ok())
            .unwrap_or(false);
        if !confirmed {
            return;
        }
        spawn(async move {
            if api::notes::delete(note.id).await.is_ok() {
                state.invalidate_notes();
                if selected.read().as_ref().map(|s| s.id) == Some(note.id) {
                    selected.set(None);
                }
            }
        });
    };

    let items = match notes.read().as_ref() {
        Some(Ok(v)) => v.clone(),
        _ => Vec::new(),
    };
    let sel = selected.read().clone();

    rsx! {
        div { class: "notes-layout",
            div { class: "notes-list",
                div { class: "notes-list-header",
                    h2 { class: "modal-title", style: "font-size:14px;", "Private Notes" }
                    button { class: "icon-btn", onclick: move |_| show_create.set(true),
                        Icon { width: 16, height: 16, icon: LdPlus }
                    }
                }
                div { style: "flex:1; overflow-y:auto;",
                    if items.is_empty() {
                        div { class: "empty-hint", style: "text-align:center;", "No notes yet" }
                    }
                    for note in items {
                        button {
                            key: "{note.id}",
                            class: if sel.as_ref().map(|s| s.id) == Some(note.id) { "note-list-item active" } else { "note-list-item" },
                            onclick: {
                                let note = note.clone();
                                move |_| selected.set(Some(note.clone()))
                            },
                            div { class: "note-list-title", "{note.title}" }
                            if let Some(repo) = &note.repo {
                                div { class: "note-list-meta",
                                    "{repo}"
                                    if let Some(n) = note.ref_number {
                                        " #{n}"
                                    }
                                }
                            }
                            div { class: "note-list-meta", "{format_date_short(&note.updated_at)}" }
                        }
                    }
                }
            }

            div { class: "note-reader",
                if let Some(note) = sel.clone() {
                    div { class: "note-reader-header",
                        div {
                            h2 { class: "modal-title", style: "font-size:14px;", "{note.title}" }
                            if let Some(repo) = &note.repo {
                                div { class: "note-list-meta",
                                    "{repo}"
                                    if let (Some(rt), Some(n)) = (note.ref_type.clone(), note.ref_number) {
                                        " · {rt} "
                                        if let Some(u) = github_url(&note) {
                                            a { href: "{u}", target: "_blank", rel: "noopener noreferrer", "#{n}" }
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "flex gap-2",
                            if let Some(u) = github_url(&note) {
                                a {
                                    class: "icon-btn",
                                    href: "{u}",
                                    target: "_blank",
                                    rel: "noopener noreferrer",
                                    title: "Open on GitHub",
                                    Icon { width: 16, height: 16, icon: LdExternalLink }
                                }
                            }
                            button {
                                class: "icon-btn",
                                onclick: {
                                    let note = note.clone();
                                    move |_| editing.set(Some(note.clone()))
                                },
                                Icon { width: 16, height: 16, icon: LdSquarePen }
                            }
                            button {
                                class: "icon-btn danger",
                                onclick: {
                                    let note = note.clone();
                                    move |_| delete_note(note.clone())
                                },
                                Icon { width: 16, height: 16, icon: LdTrash2 }
                            }
                        }
                    }
                    div { class: "note-reader-body",
                        Prose { text: note.body.clone() }
                    }
                } else {
                    div { class: "center-hint", "Select a note to read it" }
                }
            }
        }

        if show_create() {
            NoteDialog {
                on_confirm: on_create,
                on_close: move |_| show_create.set(false),
            }
        }
        if let Some(note) = editing.read().clone() {
            NoteDialog {
                note,
                on_confirm: on_update,
                on_close: move |_| editing.set(None),
            }
        }
    }
}
