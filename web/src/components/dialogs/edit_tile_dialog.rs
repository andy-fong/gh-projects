use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{LdPlus, LdTrash2, LdX};
use dioxus_free_icons::Icon;
use serde_json::{json, Value};

use crate::types::{Tile, UpdateTileInput};

#[component]
pub fn EditTileDialog(
    tile: Tile,
    on_confirm: EventHandler<UpdateTileInput>,
    on_close: EventHandler<()>,
) -> Element {
    let is_gh = tile.tile_type == "gh_query";
    let cfg_val: Value = serde_json::from_str(&tile.config).unwrap_or_else(|_| json!({}));

    let init_commands: Vec<String> = match cfg_val.get("commands").and_then(|v| v.as_array()) {
        Some(arr) if !arr.is_empty() => arr
            .iter()
            .map(|v| v.as_str().unwrap_or("").to_string())
            .collect(),
        _ => vec![cfg_val
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()],
    };
    let init_extractors = serde_json::to_string_pretty(
        cfg_val.get("field_extractors").unwrap_or(&json!({})),
    )
    .unwrap_or_else(|_| "{}".into());
    let init_variables = match cfg_val.get("variables") {
        Some(v) if v.as_object().map(|o| !o.is_empty()).unwrap_or(false) => {
            serde_json::to_string_pretty(v).unwrap_or_default()
        }
        _ => String::new(),
    };

    let mut title = use_signal(|| tile.title.clone());
    let mut commands = use_signal(|| init_commands.clone());
    let mut extractors_json = use_signal(|| init_extractors.clone());
    let mut variables_json = use_signal(|| init_variables.clone());
    let mut json_error = use_signal(|| None::<String>);

    let cfg_for_submit = cfg_val.clone();
    let submit = move |_| {
        json_error.set(None);
        if !is_gh {
            on_confirm.call(UpdateTileInput {
                title: Some(title.read().clone()),
                config: Some(cfg_for_submit.clone()),
                layout: None,
            });
            return;
        }

        let parse_obj = |s: &str, what: &str| -> Result<Value, String> {
            let s = if s.trim().is_empty() { "{}" } else { s };
            match serde_json::from_str::<Value>(s) {
                Ok(v @ Value::Object(_)) => Ok(v),
                Ok(_) => Err(format!("{what} must be a JSON object")),
                Err(e) => Err(e.to_string()),
            }
        };

        let field_extractors = match parse_obj(&extractors_json.read(), "Field extractors") {
            Ok(v) => v,
            Err(e) => {
                json_error.set(Some(e));
                return;
            }
        };
        let variables = match parse_obj(&variables_json.read(), "Variables") {
            Ok(v) => v,
            Err(e) => {
                json_error.set(Some(e));
                return;
            }
        };

        let filled: Vec<String> = commands
            .read()
            .iter()
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty())
            .collect();

        let mut new_cfg = cfg_for_submit.clone();
        if let Some(obj) = new_cfg.as_object_mut() {
            obj.insert("command".into(), json!(filled.first().cloned().unwrap_or_default()));
            obj.insert("commands".into(), json!(filled));
            obj.insert("field_extractors".into(), field_extractors);
            obj.insert("variables".into(), variables);
        }
        on_confirm.call(UpdateTileInput {
            title: Some(title.read().clone()),
            config: Some(new_cfg),
            layout: None,
        });
    };

    let cmds = commands.read().clone();

    rsx! {
        div { class: "modal-backdrop", onclick: move |_| on_close.call(()),
            div {
                class: "modal",
                style: "width: 560px;",
                onclick: move |e| e.stop_propagation(),
                div { class: "modal-header",
                    h2 { class: "modal-title", "Edit Tile" }
                    button { class: "icon-btn", onclick: move |_| on_close.call(()),
                        Icon { width: 16, height: 16, icon: LdX }
                    }
                }
                form { onsubmit: submit,
                    div { class: "field",
                        label { "Title" }
                        input {
                            class: "input",
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
                        }
                        div { class: "field",
                            label { "Variables  — JSON object; array value runs the command once per element" }
                            textarea {
                                class: "textarea mono",
                                rows: 3,
                                spellcheck: false,
                                value: "{variables_json}",
                                oninput: move |e| {
                                    variables_json.set(e.value());
                                    json_error.set(None);
                                },
                            }
                        }
                        div { class: "field",
                            label { "Field extractors override  — leave empty to use global settings" }
                            textarea {
                                class: "textarea mono",
                                rows: 3,
                                spellcheck: false,
                                value: "{extractors_json}",
                                oninput: move |e| {
                                    extractors_json.set(e.value());
                                    json_error.set(None);
                                },
                            }
                            if let Some(err) = json_error() {
                                p { class: "error-text", "{err}" }
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
                        button { class: "btn btn-primary", r#type: "submit", "Save" }
                    }
                }
            }
        }
    }
}
