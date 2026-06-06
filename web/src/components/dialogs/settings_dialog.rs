use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::LdX;
use dioxus_free_icons::Icon;
use serde_json::Value;
use std::collections::BTreeMap;

use crate::api;
use crate::field_extractors::default_field_extractors;
use crate::platform;
use crate::state::use_app_state;

#[component]
pub fn SettingsDialog(on_close: EventHandler<()>) -> Element {
    let state = use_app_state();

    let init_json = serde_json::to_string_pretty(&*state.user_extractors.read())
        .unwrap_or_else(|_| "{}".into());
    let mut extractors_json = use_signal(|| init_json.clone());
    let mut json_error = use_signal(|| None::<String>);
    let mut exporting = use_signal(|| false);
    let mut restoring = use_signal(|| false);
    let mut restore_error = use_signal(|| None::<String>);
    let mut restore_success = use_signal(|| false);

    let save = move |_| {
        let s = extractors_json.read().clone();
        let s = if s.trim().is_empty() { "{}".to_string() } else { s };
        match serde_json::from_str::<Value>(&s) {
            Ok(Value::Object(_)) => match serde_json::from_str::<BTreeMap<String, String>>(&s) {
                Ok(map) => {
                    state.set_user_extractors(map);
                    json_error.set(None);
                    on_close.call(());
                }
                Err(e) => json_error.set(Some(e.to_string())),
            },
            Ok(_) => json_error.set(Some("Must be a JSON object".into())),
            Err(e) => json_error.set(Some(e.to_string())),
        }
    };

    let export = move |_| {
        exporting.set(true);
        spawn(async move {
            if let Ok(mut data) = api::backup::export().await {
                // Enrich each tile with its localStorage column state.
                if let Some(dashboards) = data.get_mut("dashboards").and_then(|d| d.as_array_mut()) {
                    for dash in dashboards.iter_mut() {
                        if let Some(tiles) = dash.get_mut("tiles").and_then(|t| t.as_array_mut()) {
                            for tile in tiles.iter_mut() {
                                let id = tile.get("id").and_then(|i| i.as_i64()).unwrap_or(0);
                                if let Some(h) = platform::get_list(&format!("gh-tile-hidden-{id}")) {
                                    tile["hidden_cols"] = serde_json::json!(h);
                                }
                                if let Some(o) = platform::get_list(&format!("gh-tile-col-order-{id}")) {
                                    tile["col_order"] = serde_json::json!(o);
                                }
                            }
                        }
                    }
                }
                let filename = format!(
                    "gh_project_backup_{}.json",
                    js_sys::Date::new_0()
                        .to_iso_string()
                        .as_string()
                        .unwrap_or_default()
                        .replace(['-', ':'], "")
                        .chars()
                        .take(13)
                        .collect::<String>()
                );
                let content = serde_json::to_string_pretty(&data).unwrap_or_default();
                platform::download_text(&filename, &content);
            }
            exporting.set(false);
        });
    };

    let restore = move |e: Event<FormData>| {
        let files = e.files();
        let Some(file) = files.into_iter().next() else { return };
        restoring.set(true);
        restore_error.set(None);
        restore_success.set(false);
        spawn(async move {
            let text = match file.read_string().await {
                Ok(t) => t,
                Err(_) => {
                    restore_error.set(Some("Could not read file".into()));
                    restoring.set(false);
                    return;
                }
            };
            let data: Value = match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(err) => {
                    restore_error.set(Some(err.to_string()));
                    restoring.set(false);
                    return;
                }
            };
            match api::backup::restore(&data).await {
                Ok(result) => {
                    // Clear stale per-tile localStorage, then rewrite for new IDs.
                    for k in platform::keys_with_prefix(&["gh-tile-hidden-", "gh-tile-col-order-"]) {
                        platform::remove_key(&k);
                    }
                    let result_dashboards =
                        result.get("dashboards").and_then(|d| d.as_array());
                    let data_dashboards = data.get("dashboards").and_then(|d| d.as_array());
                    if let (Some(rd), Some(dd)) = (result_dashboards, data_dashboards) {
                        for (di, rdash) in rd.iter().enumerate() {
                            let tile_ids = rdash
                                .get("tile_ids")
                                .and_then(|t| t.as_array())
                                .cloned()
                                .unwrap_or_default();
                            let tiles = dd
                                .get(di)
                                .and_then(|d| d.get("tiles"))
                                .and_then(|t| t.as_array())
                                .cloned()
                                .unwrap_or_default();
                            for (ti, new_id) in tile_ids.iter().enumerate() {
                                let new_id = new_id.as_i64().unwrap_or(0);
                                if let Some(tile) = tiles.get(ti) {
                                    if let Some(h) = tile.get("hidden_cols").and_then(|v| v.as_array()) {
                                        let v: Vec<String> = h
                                            .iter()
                                            .filter_map(|x| x.as_str().map(String::from))
                                            .collect();
                                        platform::set_list(&format!("gh-tile-hidden-{new_id}"), &v);
                                    }
                                    if let Some(o) = tile.get("col_order").and_then(|v| v.as_array()) {
                                        let v: Vec<String> = o
                                            .iter()
                                            .filter_map(|x| x.as_str().map(String::from))
                                            .collect();
                                        platform::set_list(&format!("gh-tile-col-order-{new_id}"), &v);
                                    }
                                }
                            }
                        }
                    }
                    restore_success.set(true);
                }
                Err(err) => restore_error.set(Some(err)),
            }
            restoring.set(false);
        });
    };

    let defaults = serde_json::to_string_pretty(&default_field_extractors()).unwrap_or_default();

    rsx! {
        div { class: "modal-backdrop", onclick: move |_| on_close.call(()),
            div {
                class: "modal",
                style: "width: 520px;",
                onclick: move |e| e.stop_propagation(),
                div { class: "modal-header",
                    h2 { class: "modal-title", "Global Settings" }
                    button { class: "icon-btn", onclick: move |_| on_close.call(()),
                        Icon { width: 16, height: 16, icon: LdX }
                    }
                }
                form { onsubmit: save,
                    div { class: "field",
                        label { "Field extractors  — maps column → sub-field to display" }
                        textarea {
                            class: "textarea mono",
                            rows: 6,
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
                        p { class: "hint",
                            "Use | for fallbacks: \"name|login\" tries name first, falls back to login if empty."
                        }
                        p { class: "hint", "Built-in defaults (always applied unless overridden):" }
                        pre { class: "hint mono", "{defaults}" }
                    }

                    div { style: "border-top: 1px solid var(--border); padding-top: 16px;",
                        p { class: "hint", style: "margin-bottom: 12px;", "Backup & Restore" }
                        div { class: "flex gap-2",
                            button {
                                r#type: "button",
                                class: "btn",
                                disabled: exporting(),
                                onclick: export,
                                if exporting() { "Exporting…" } else { "Export backup" }
                            }
                            label { class: "btn",
                                if restoring() { "Restoring…" } else { "Restore backup" }
                                input {
                                    r#type: "file",
                                    accept: ".json",
                                    style: "display:none;",
                                    disabled: restoring(),
                                    onchange: restore,
                                }
                            }
                        }
                        if let Some(err) = restore_error() {
                            p { class: "error-text", "{err}" }
                        }
                        if restore_success() {
                            p { style: "color: var(--good); font-size: 12px; margin-top: 8px;",
                                "Restore successful — reload the page to see your data."
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
