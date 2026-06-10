use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{LdPlus, LdX};
use dioxus_free_icons::Icon;

use crate::types::{ChecklistItem, CreateItemInput, WarRoomItem};
use crate::war_room::STAGES;

/// Create/edit a war room item. Emits a `CreateItemInput` carrying the full
/// field set; the page maps it to a create or (replace-style) update.
///
/// `siblings` are other items in the same room, offered as "depends on" options.
#[component]
pub fn WarRoomItemDialog(
    #[props(default)] item: Option<WarRoomItem>,
    repo: Option<String>,
    siblings: Vec<(i64, String)>,
    on_confirm: EventHandler<CreateItemInput>,
    on_close: EventHandler<()>,
) -> Element {
    let is_edit = item.is_some();

    let mut label = use_signal(|| item.as_ref().map(|i| i.label.clone()).unwrap_or_default());
    let mut stage = use_signal(|| {
        item.as_ref()
            .map(|i| i.stage.clone())
            .unwrap_or_else(|| "todo".to_string())
    });
    let mut ref_type = use_signal(|| {
        item.as_ref()
            .and_then(|i| i.ref_type.clone())
            .unwrap_or_default()
    });
    let mut ref_number = use_signal(|| {
        item.as_ref()
            .and_then(|i| i.ref_number)
            .map(|n| n.to_string())
            .unwrap_or_default()
    });
    let mut note = use_signal(|| item.as_ref().map(|i| i.note.clone()).unwrap_or_default());
    let mut depends_on = use_signal(|| {
        item.as_ref()
            .and_then(|i| i.depends_on)
            .map(|n| n.to_string())
            .unwrap_or_default()
    });
    let mut checklist = use_signal(|| {
        item.as_ref()
            .map(|i| i.checklist_items())
            .unwrap_or_default()
    });
    let mut new_step = use_signal(String::new);

    let current_id = item.as_ref().map(|i| i.id);

    let add_step = move || {
        let text = new_step.read().trim().to_string();
        if text.is_empty() {
            return;
        }
        checklist.write().push(ChecklistItem { text, done: false });
        new_step.set(String::new());
    };

    let submit = move |_| {
        let label_v = label.read().trim().to_string();
        if label_v.is_empty() {
            return;
        }
        let rt = ref_type.read().trim().to_string();
        let dep = depends_on.read().trim().to_string();
        let input = CreateItemInput {
            label: label_v,
            ref_type: (!rt.is_empty()).then_some(rt),
            ref_number: ref_number.read().trim().parse::<i64>().ok(),
            note: Some(note.read().clone()),
            stage: Some(stage.read().clone()),
            checklist: Some(checklist.read().clone()),
            depends_on: dep.parse::<i64>().ok(),
            source_item_id: None,
        };
        on_confirm.call(input);
    };

    rsx! {
        div { class: "modal-backdrop", onclick: move |_| on_close.call(()),
            div {
                class: "modal",
                style: "width: 560px;",
                onclick: move |e| e.stop_propagation(),
                div { class: "modal-header",
                    h2 { class: "modal-title", if is_edit { "Edit item" } else { "New item" } }
                    button { class: "icon-btn", onclick: move |_| on_close.call(()),
                        Icon { width: 16, height: 16, icon: LdX }
                    }
                }
                form { onsubmit: submit,
                    div { class: "row-group",
                        div {
                            label { class: "form-label", "Title" }
                            input {
                                class: "input",
                                required: true,
                                placeholder: "v2.3.2",
                                value: "{label}",
                                oninput: move |e| label.set(e.value()),
                            }
                        }
                        div {
                            label { class: "form-label", "Stage" }
                            select {
                                class: "select",
                                value: "{stage}",
                                onchange: move |e| stage.set(e.value()),
                                for (k , l) in STAGES.iter() {
                                    option { value: "{k}", selected: *k == stage.read().as_str(), "{l}" }
                                }
                            }
                        }
                        div {
                            label { class: "form-label", "Depends on" }
                            select {
                                class: "select",
                                value: "{depends_on}",
                                onchange: move |e| depends_on.set(e.value()),
                                option { value: "", selected: depends_on.read().is_empty(), "—" }
                                for (sid , slabel) in siblings.iter() {
                                    if Some(*sid) != current_id {
                                        option {
                                            value: "{sid}",
                                            selected: sid.to_string() == *depends_on.read(),
                                            "{slabel}"
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div { class: "row-group",
                        div {
                            label { class: "form-label", "Ref type" }
                            select {
                                class: "select",
                                value: "{ref_type}",
                                onchange: move |e| ref_type.set(e.value()),
                                option { value: "", selected: ref_type.read().is_empty(), "None" }
                                option { value: "pr", selected: *ref_type.read() == "pr", "PR" }
                                option { value: "issue", selected: *ref_type.read() == "issue", "Issue" }
                            }
                        }
                        div {
                            label { class: "form-label", "Issue/PR #" }
                            input {
                                class: "input",
                                placeholder: "123",
                                value: "{ref_number}",
                                oninput: move |e| ref_number.set(e.value()),
                            }
                        }
                        div {
                            label { class: "form-label", "Repo (from group)" }
                            input {
                                class: "input",
                                disabled: true,
                                value: repo.clone().unwrap_or_else(|| "—".to_string()),
                            }
                        }
                    }

                    div { class: "field",
                        label { "Note" }
                        textarea {
                            class: "textarea",
                            rows: 3,
                            placeholder: "envoy upgrade PR merged, go version bump, release building…",
                            value: "{note}",
                            oninput: move |e| note.set(e.value()),
                        }
                    }

                    div { class: "field",
                        label { "Checklist" }
                        for (idx , step) in checklist.read().clone().iter().enumerate() {
                            div { class: "wr-step-edit",
                                input {
                                    r#type: "checkbox",
                                    checked: step.done,
                                    onchange: move |e| {
                                        if let Some(s) = checklist.write().get_mut(idx) {
                                            s.done = e.checked();
                                        }
                                    },
                                }
                                input {
                                    class: "input",
                                    value: "{step.text}",
                                    oninput: move |e| {
                                        if let Some(s) = checklist.write().get_mut(idx) {
                                            s.text = e.value();
                                        }
                                    },
                                }
                                button {
                                    class: "icon-btn danger",
                                    r#type: "button",
                                    onclick: move |_| {
                                        checklist.write().remove(idx);
                                    },
                                    Icon { width: 14, height: 14, icon: LdX }
                                }
                            }
                        }
                        div { class: "wr-step-edit",
                            input {
                                class: "input",
                                placeholder: "Add a step…",
                                value: "{new_step}",
                                oninput: move |e| new_step.set(e.value()),
                                onkeydown: {
                                    let mut add = add_step;
                                    move |e: Event<KeyboardData>| {
                                        if e.key() == Key::Enter {
                                            e.prevent_default();
                                            add();
                                        }
                                    }
                                },
                            }
                            button {
                                class: "icon-btn",
                                r#type: "button",
                                onclick: {
                                    let mut add = add_step;
                                    move |_| add()
                                },
                                Icon { width: 14, height: 14, icon: LdPlus }
                            }
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
                            if is_edit { "Save" } else { "Add item" }
                        }
                    }
                }
            }
        }
    }
}
