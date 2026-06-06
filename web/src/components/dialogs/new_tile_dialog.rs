use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{LdPlus, LdTrash2, LdX};
use dioxus_free_icons::Icon;
use std::collections::BTreeMap;

use crate::types::{CreateTileInput, GhQueryConfig, NoteConfig, VarValue};

/// Prefill values when "pasting" a copied tile. Mirrors `NewTileInitialValues`.
#[derive(Clone, PartialEq, Default)]
pub struct NewTileInit {
    pub tile_type: String, // "gh_query" | "note"
    pub title: String,
    pub commands: Option<Vec<String>>,
    pub variables_json: Option<String>,
    pub note_id: Option<String>,
}

#[component]
pub fn NewTileDialog(
    #[props(default)] initial: Option<NewTileInit>,
    on_confirm: EventHandler<CreateTileInput>,
    on_close: EventHandler<()>,
) -> Element {
    let is_paste = initial.is_some();
    let init = initial.clone().unwrap_or_default();

    let mut tile_type = use_signal(|| {
        if init.tile_type.is_empty() {
            "gh_query".to_string()
        } else {
            init.tile_type.clone()
        }
    });
    let mut title = use_signal(|| init.title.clone());
    let mut commands = use_signal(|| init.commands.clone().unwrap_or_else(|| vec![String::new()]));
    let mut note_id = use_signal(|| init.note_id.clone().unwrap_or_default());
    let mut variables_json = use_signal(|| init.variables_json.clone().unwrap_or_default());
    let mut json_error = use_signal(|| None::<String>);

    let submit = move |_| {
        json_error.set(None);
        if tile_type.read().as_str() == "gh_query" {
            let filled: Vec<String> = commands
                .read()
                .iter()
                .map(|c| c.trim().to_string())
                .filter(|c| !c.is_empty())
                .collect();
            if filled.is_empty() {
                return;
            }
            let vars_str = variables_json.read().trim().to_string();
            let variables: BTreeMap<String, VarValue> = if vars_str.is_empty() {
                BTreeMap::new()
            } else {
                match serde_json::from_str::<serde_json::Value>(&vars_str) {
                    Ok(serde_json::Value::Object(_)) => {
                        match serde_json::from_str(&vars_str) {
                            Ok(m) => m,
                            Err(e) => {
                                json_error.set(Some(e.to_string()));
                                return;
                            }
                        }
                    }
                    Ok(_) => {
                        json_error.set(Some("Must be a JSON object".into()));
                        return;
                    }
                    Err(e) => {
                        json_error.set(Some(e.to_string()));
                        return;
                    }
                }
            };
            let config = GhQueryConfig {
                command: filled[0].clone(),
                commands: Some(filled),
                columns: None,
                field_extractors: None,
                variables: Some(variables),
            };
            let t = title.read().trim().to_string();
            on_confirm.call(CreateTileInput {
                title: if t.is_empty() { "GH Query".into() } else { t },
                tile_type: "gh_query".into(),
                config: serde_json::to_value(config).unwrap_or_default(),
                layout: None,
            });
        } else {
            let nid: i64 = note_id.read().trim().parse().unwrap_or(0);
            let t = title.read().trim().to_string();
            on_confirm.call(CreateTileInput {
                title: if t.is_empty() { "Note".into() } else { t },
                tile_type: "note".into(),
                config: serde_json::to_value(NoteConfig { note_id: nid }).unwrap_or_default(),
                layout: None,
            });
        }
    };

    let cmds = commands.read().clone();
    let is_gh = tile_type.read().as_str() == "gh_query";

    rsx! {
        div { class: "modal-backdrop", onclick: move |_| on_close.call(()),
            div {
                class: "modal",
                style: "width: 560px;",
                onclick: move |e| e.stop_propagation(),
                div { class: "modal-header",
                    h2 { class: "modal-title", if is_paste { "Paste Tile" } else { "Add Tile" } }
                    button { class: "icon-btn", onclick: move |_| on_close.call(()),
                        Icon { width: 16, height: 16, icon: LdX }
                    }
                }
                form { onsubmit: submit,
                    div { class: "field",
                        label { "Tile type" }
                        div { class: "flex gap-2",
                            button {
                                r#type: "button",
                                class: if is_gh { "btn btn-primary btn-sm" } else { "btn btn-sm" },
                                onclick: move |_| tile_type.set("gh_query".into()),
                                "GH Query"
                            }
                            button {
                                r#type: "button",
                                class: if !is_gh { "btn btn-primary btn-sm" } else { "btn btn-sm" },
                                onclick: move |_| tile_type.set("note".into()),
                                "Note"
                            }
                        }
                    }
                    div { class: "field",
                        label { "Title" }
                        input {
                            class: "input",
                            placeholder: if is_gh { "Open Issues" } else { "My Note" },
                            value: "{title}",
                            oninput: move |e| title.set(e.value()),
                        }
                    }
                    if is_gh {
                        div { class: "field",
                            label { "gh command(s)  — multiple commands are merged into one table" }
                            for (i , cmd) in cmds.iter().enumerate() {
                                div { key: "{i}", class: "flex gap-2 items-center", style: "margin-bottom:8px;",
                                    input {
                                        class: "input mono",
                                        placeholder: "issue list --repo owner/repo --json number,title,state,url",
                                        value: "{cmd}",
                                        oninput: move |e| {
                                            commands.write()[i] = e.value();
                                        },
                                    }
                                    if cmds.len() > 1 {
                                        button {
                                            r#type: "button",
                                            class: "icon-btn danger",
                                            onclick: move |_| {
                                                commands.write().remove(i);
                                            },
                                            Icon { width: 14, height: 14, icon: LdTrash2 }
                                        }
                                    }
                                }
                            }
                            button {
                                r#type: "button",
                                class: "chip",
                                onclick: move |_| commands.write().push(String::new()),
                                Icon { width: 12, height: 12, icon: LdPlus }
                                "Add command"
                            }
                            p { class: "hint",
                                "Omit the leading \"gh \". Use {{var}} for template variables. Click any cell value to filter by it."
                            }
                        }
                        div { class: "field",
                            label { "Variables  — JSON object; array value runs the command once per element" }
                            textarea {
                                class: "textarea mono",
                                rows: 3,
                                spellcheck: false,
                                placeholder: "{{\n  \"repos\": [\"owner/repo-a\", \"owner/repo-b\"]\n}}",
                                value: "{variables_json}",
                                oninput: move |e| {
                                    variables_json.set(e.value());
                                    json_error.set(None);
                                },
                            }
                            if let Some(err) = json_error() {
                                p { class: "error-text", "{err}" }
                            }
                        }
                    } else {
                        div { class: "field",
                            label { "Note ID" }
                            input {
                                class: "input",
                                r#type: "number",
                                placeholder: "1",
                                required: true,
                                value: "{note_id}",
                                oninput: move |e| note_id.set(e.value()),
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
                        button { class: "btn btn-primary", r#type: "submit", "Add tile" }
                    }
                }
            }
        }
    }
}
