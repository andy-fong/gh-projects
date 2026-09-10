use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::LdX;
use dioxus_free_icons::Icon;
use serde_json::Value;
use std::collections::BTreeMap;

use crate::api;
use crate::components::gh_query_tile::{
    colorder_key, hidden_key, COLORDER_KEY_PREFIX, HIDDEN_KEY_PREFIX,
};
use crate::field_extractors::default_field_extractors;
use crate::platform;
use crate::state::use_app_state;
use crate::war_room::STAGES;

#[component]
pub fn SettingsDialog(on_close: EventHandler<()>) -> Element {
    let state = use_app_state();

    let init_json = serde_json::to_string_pretty(&*state.user_extractors.read())
        .unwrap_or_else(|_| "{}".into());
    let mut extractors_json = use_signal(|| init_json.clone());
    let mut stage_emojis = use_signal(|| state.stage_emojis());
    let mut json_error = use_signal(|| None::<String>);
    let mut exporting = use_signal(|| false);
    let mut restoring = use_signal(|| false);
    let mut restore_error = use_signal(|| None::<String>);
    let mut restore_success = use_signal(|| false);
    let mut restore_report = use_signal(String::new);

    let save = move |_| {
        let s = extractors_json.read().clone();
        let s = if s.trim().is_empty() { "{}".to_string() } else { s };
        match serde_json::from_str::<Value>(&s) {
            Ok(Value::Object(_)) => match serde_json::from_str::<BTreeMap<String, String>>(&s) {
                Ok(map) => {
                    state.set_user_extractors(map);
                    state.set_user_stage_emojis(stage_emojis.read().clone());
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
                                if let Some(h) = platform::get_list(&hidden_key(id)) {
                                    tile["hidden_cols"] = serde_json::json!(h);
                                }
                                if let Some(o) = platform::get_list(&colorder_key(id)) {
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
        restore_report.set(String::new());
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
                    for k in platform::keys_with_prefix(&[HIDDEN_KEY_PREFIX, COLORDER_KEY_PREFIX]) {
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
                                        platform::set_list(&hidden_key(new_id), &v);
                                    }
                                    if let Some(o) = tile.get("col_order").and_then(|v| v.as_array()) {
                                        let v: Vec<String> = o
                                            .iter()
                                            .filter_map(|x| x.as_str().map(String::from))
                                            .collect();
                                        platform::set_list(&colorder_key(new_id), &v);
                                    }
                                }
                            }
                        }
                    }
                    // Surface what the backend actually did — dropped links or
                    // skipped release watches are worth knowing about.
                    let n = |k: &str| result.get(k).and_then(|v| v.as_u64()).unwrap_or(0);
                    let mut parts = vec![format!("{} link(s) restored", n("links_restored"))];
                    if n("links_dropped") > 0 {
                        parts.push(format!("{} link(s) dropped — target missing from the backup", n("links_dropped")));
                    }
                    if n("release_watches_skipped") > 0 {
                        parts.push(format!("{} release watch(es) skipped — repo not found", n("release_watches_skipped")));
                    }
                    if let Some(snap) = result.get("snapshot").and_then(|v| v.as_str()) {
                        parts.push(format!("previous database saved to {snap}"));
                    }
                    restore_report.set(parts.join(" · "));
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

                    div { class: "field",
                        label { "Status emojis — used in the Slack update for each stage" }
                        for &(key , stage_label) in STAGES.iter() {
                            div { class: "wr-repo-row",
                                span { style: "flex:0 0 110px; color:var(--muted);", "{stage_label}" }
                                input {
                                    class: "input",
                                    spellcheck: false,
                                    placeholder: ":emoji:",
                                    value: stage_emojis.read().get(key).cloned().unwrap_or_default(),
                                    oninput: move |e| {
                                        stage_emojis.write().insert(key.to_string(), e.value());
                                    },
                                }
                            }
                        }
                        p { class: "hint", "Use your workspace's Slack emoji codes, e.g. :white_check_mark:" }
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
                            if !restore_report.read().is_empty() {
                                p { class: "hint", style: "margin-top: 4px; word-break: break-all;",
                                    "{restore_report}"
                                }
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
