use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::LdX;
use dioxus_free_icons::Icon;

use crate::types::{CreateGroupInput, Repo, WarRoomGroup};

/// Create/edit a war room group. Picking a registry repo prefills the group
/// name + `owner/repo` (both still editable). Emits a `CreateGroupInput`.
#[component]
pub fn WarRoomGroupDialog(
    #[props(default)] group: Option<WarRoomGroup>,
    repos: Vec<Repo>,
    on_confirm: EventHandler<CreateGroupInput>,
    on_close: EventHandler<()>,
) -> Element {
    let is_edit = group.is_some();

    let mut repo_id = use_signal(|| {
        group
            .as_ref()
            .and_then(|g| g.repo_id)
            .map(|n| n.to_string())
            .unwrap_or_default()
    });
    let mut name = use_signal(|| group.as_ref().map(|g| g.name.clone()).unwrap_or_default());
    let mut repo = use_signal(|| {
        group
            .as_ref()
            .and_then(|g| g.repo.clone())
            .unwrap_or_default()
    });

    let repos_for_select = repos.clone();
    let on_pick = move |e: Event<FormData>| {
        let val = e.value();
        repo_id.set(val.clone());
        if let Ok(id) = val.parse::<i64>() {
            if let Some(r) = repos_for_select.iter().find(|r| r.id == id) {
                // Prefill from the registry; only fill name if empty so we don't
                // clobber a custom label the user already typed.
                if name.read().trim().is_empty() {
                    name.set(r.name.clone());
                }
                repo.set(r.owner_repo.clone());
            }
        }
    };

    let submit = move |_| {
        let name_v = name.read().trim().to_string();
        if name_v.is_empty() {
            return;
        }
        let repo_v = repo.read().trim().to_string();
        let input = CreateGroupInput {
            name: name_v,
            repo_id: repo_id.read().trim().parse::<i64>().ok(),
            repo: (!repo_v.is_empty()).then_some(repo_v),
        };
        on_confirm.call(input);
    };

    rsx! {
        div { class: "modal-backdrop", onclick: move |_| on_close.call(()),
            div {
                class: "modal",
                style: "width: 460px;",
                onclick: move |e| e.stop_propagation(),
                div { class: "modal-header",
                    h2 { class: "modal-title", if is_edit { "Edit group" } else { "New group" } }
                    button { class: "icon-btn", onclick: move |_| on_close.call(()),
                        Icon { width: 16, height: 16, icon: LdX }
                    }
                }
                form { onsubmit: submit,
                    div { class: "field",
                        label { "Repo (from registry)" }
                        select {
                            class: "select",
                            value: "{repo_id}",
                            onchange: on_pick,
                            option { value: "", "Custom / none" }
                            for r in repos.iter() {
                                option { value: "{r.id}", "{r.name} ({r.owner_repo})" }
                            }
                        }
                        div { class: "hint", "Items in this group are auto-scoped to this repo." }
                    }
                    div { class: "field",
                        label { "Group name" }
                        input {
                            class: "input",
                            required: true,
                            placeholder: "Kgateway OSS",
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                        }
                    }
                    div { class: "field",
                        label { "Repo (owner/repo)" }
                        input {
                            class: "input",
                            placeholder: "kgateway-dev/kgateway",
                            value: "{repo}",
                            oninput: move |e| repo.set(e.value()),
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
                            if is_edit { "Save" } else { "Add group" }
                        }
                    }
                }
            }
        }
    }
}
