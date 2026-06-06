use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::LdX;
use dioxus_free_icons::Icon;

use crate::types::{CreateNoteInput, Note};

/// Create/edit a private note. Mirrors `NoteDialog.tsx`.
///
/// `note` (edit mode) takes precedence over `initial` (prefilled create, e.g.
/// from a table row). `on_confirm` receives the assembled input.
#[component]
pub fn NoteDialog(
    #[props(default)] note: Option<Note>,
    #[props(default)] initial: Option<CreateNoteInput>,
    on_confirm: EventHandler<CreateNoteInput>,
    on_close: EventHandler<()>,
) -> Element {
    let is_edit = note.is_some();

    let (i_title, i_body, i_repo, i_ref_type, i_ref_num) = if let Some(n) = &note {
        (
            n.title.clone(),
            n.body.clone(),
            n.repo.clone().unwrap_or_default(),
            n.ref_type.clone().unwrap_or_default(),
            n.ref_number.map(|x| x.to_string()).unwrap_or_default(),
        )
    } else if let Some(i) = &initial {
        (
            i.title.clone(),
            i.body.clone().unwrap_or_default(),
            i.repo.clone().unwrap_or_default(),
            i.ref_type.clone().unwrap_or_default(),
            i.ref_number.map(|x| x.to_string()).unwrap_or_default(),
        )
    } else {
        Default::default()
    };

    let mut title = use_signal(|| i_title.clone());
    let mut body = use_signal(|| i_body.clone());
    let mut repo = use_signal(|| i_repo.clone());
    let mut ref_type = use_signal(|| i_ref_type.clone());
    let mut ref_number = use_signal(|| i_ref_num.clone());

    let submit = move |_| {
        let repo_v = repo.read().trim().to_string();
        let ref_type_v = ref_type.read().trim().to_string();
        let ref_num_v = ref_number.read().trim().to_string();
        let input = CreateNoteInput {
            title: title.read().clone(),
            body: Some(body.read().clone()),
            repo: (!repo_v.is_empty()).then_some(repo_v),
            ref_type: (!ref_type_v.is_empty()).then_some(ref_type_v),
            ref_number: ref_num_v.parse::<i64>().ok(),
            tags: None,
        };
        on_confirm.call(input);
    };

    rsx! {
        div { class: "modal-backdrop", onclick: move |_| on_close.call(()),
            div {
                class: "modal",
                style: "width: 600px;",
                onclick: move |e| e.stop_propagation(),
                div { class: "modal-header",
                    h2 { class: "modal-title", if is_edit { "Edit Note" } else { "New Note" } }
                    button { class: "icon-btn", onclick: move |_| on_close.call(()),
                        Icon { width: 16, height: 16, icon: LdX }
                    }
                }
                form {
                    onsubmit: submit,
                    div { class: "field",
                        label { "Title" }
                        input {
                            class: "input",
                            required: true,
                            value: "{title}",
                            oninput: move |e| title.set(e.value()),
                        }
                    }
                    div { class: "row-group",
                        div {
                            label { class: "form-label", "Repo (owner/repo)" }
                            input {
                                class: "input",
                                placeholder: "org/repo",
                                value: "{repo}",
                                oninput: move |e| repo.set(e.value()),
                            }
                        }
                        div {
                            label { class: "form-label", "Ref type" }
                            select {
                                class: "select",
                                value: "{ref_type}",
                                onchange: move |e| ref_type.set(e.value()),
                                option { value: "", "None" }
                                option { value: "issue", "Issue" }
                                option { value: "pr", "PR" }
                                option { value: "repo", "Repo" }
                            }
                        }
                        div {
                            label { class: "form-label", "Issue/PR #" }
                            input {
                                class: "input",
                                r#type: "number",
                                placeholder: "123",
                                value: "{ref_number}",
                                oninput: move |e| ref_number.set(e.value()),
                            }
                        }
                    }
                    div { class: "field",
                        label { "Body (Markdown)" }
                        textarea {
                            class: "textarea mono",
                            rows: 10,
                            placeholder: "Write your private note here…",
                            value: "{body}",
                            oninput: move |e| body.set(e.value()),
                        }
                    }
                    div { class: "form-actions",
                        button {
                            class: "btn btn-ghost",
                            r#type: "button",
                            onclick: move |_| on_close.call(()),
                            "Cancel"
                        }
                        button { class: "btn btn-primary", r#type: "submit",
                            if is_edit { "Save" } else { "Create" }
                        }
                    }
                }
            }
        }
    }
}
