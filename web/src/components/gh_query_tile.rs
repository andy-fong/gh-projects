use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{
    LdChevronDown, LdChevronUp, LdChevronsUpDown, LdColumns2, LdFilter, LdGripVertical,
    LdListOrdered, LdSearch, LdStickyNote, LdX,
};
use dioxus_free_icons::Icon;
use serde_json::{json, Value};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::api;
use crate::components::common::Tooltip;
use crate::components::detail_panel::DetailPanel;
use crate::components::dialogs::note_dialog::NoteDialog;
use crate::field_extractors::{extract_display, format_date_only, is_datetime_string};
use crate::platform;
use crate::state::use_app_state;
use crate::types::{CreateNoteInput, GhQueryConfig, Note, VarValue};

// ---------- pure helpers ----------

fn expand_commands(config: &GhQueryConfig) -> Vec<String> {
    let base: Vec<String> = match &config.commands {
        Some(c) if !c.is_empty() => c.clone(),
        _ => vec![config.command.clone()],
    };
    let vars = config.variables.clone().unwrap_or_default();
    let array_entry = vars.iter().find_map(|(k, v)| match v {
        VarValue::Many(list) => Some((k.clone(), list.clone())),
        _ => None,
    });
    base.into_iter()
        .flat_map(|cmd| {
            let mut expanded = cmd;
            for (k, v) in vars.iter() {
                if let VarValue::One(val) = v {
                    expanded = expanded.replace(&format!("{{{{{k}}}}}"), val);
                }
            }
            if let Some((key, values)) = &array_entry {
                values
                    .iter()
                    .map(|val| expanded.replace(&format!("{{{{{key}}}}}"), val))
                    .collect::<Vec<_>>()
            } else {
                vec![expanded]
            }
        })
        .collect()
}

fn parse_repo_from_command(cmd: &str) -> Option<String> {
    let toks: Vec<&str> = cmd.split_whitespace().collect();
    for (i, t) in toks.iter().enumerate() {
        if (*t == "--repo" || *t == "-R") && i + 1 < toks.len() {
            return Some(toks[i + 1].to_string());
        }
        if let Some(rest) = t.strip_prefix("--repo=") {
            return Some(rest.to_string());
        }
    }
    None
}

fn row_ref_type(row: &Value, commands: &[String]) -> &'static str {
    if let Some(b) = row.get("isPullRequest").and_then(|v| v.as_bool()) {
        return if b { "pr" } else { "issue" };
    }
    let cmd = commands
        .first()
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    if cmd.starts_with("pr ") || cmd.contains(" prs ") || cmd.starts_with("prs ") {
        "pr"
    } else {
        "issue"
    }
}

fn row_repo(row: &Value) -> Option<String> {
    let repo = row.get("repository")?;
    if repo.is_null() {
        return None;
    }
    if let Some(s) = repo.as_str() {
        return Some(s.to_string());
    }
    if let Some(o) = repo.as_object() {
        return o
            .get("nameWithOwner")
            .or_else(|| o.get("name"))
            .and_then(|v| v.as_str())
            .map(String::from);
    }
    None
}

fn row_number(row: &Value) -> Option<i64> {
    let n = row.get("number")?;
    n.as_i64()
        .or_else(|| n.as_str().and_then(|s| s.parse().ok()))
}

fn row_key(row: &Value) -> Option<String> {
    let num = row_number(row)?;
    let repo = row_repo(row)?;
    let name = repo.rsplit('/').next().unwrap_or(&repo);
    Some(format!("{name}#{num}"))
}

fn cmp_vals(a: &str, b: &str) -> Ordering {
    match (a.parse::<f64>(), b.parse::<f64>()) {
        (Ok(x), Ok(y)) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
        _ => a.to_lowercase().cmp(&b.to_lowercase()),
    }
}

fn hidden_key(tile_id: i64) -> String {
    format!("gh-tile-hidden-{tile_id}")
}

fn ex_of<'a>(extractors: &'a BTreeMap<String, String>, k: &str) -> Option<&'a str> {
    extractors.get(k).map(|s| s.as_str())
}

// ---------- component ----------

#[component]
pub fn GhQueryTile(config: GhQueryConfig, tile_id: i64) -> Element {
    let state = use_app_state();

    // Keep command list current even when the tile is edited in place.
    let cmds = expand_commands(&config);
    let mut cmds_sig = use_signal(|| cmds.clone());
    if *cmds_sig.peek() != cmds {
        cmds_sig.set(cmds.clone());
    }

    let gh = use_resource(move || {
        let _ = state.gh_ver.read();
        let commands = cmds_sig.read().clone();
        async move {
            let mut out: Vec<(String, Result<Value, String>)> = Vec::new();
            for c in commands {
                let r = api::gh::execute(&c).await.map(|resp| resp.output);
                out.push((c, r));
            }
            out
        }
    });

    let all_notes = use_resource(move || {
        let _ = state.notes_ver.read();
        async move { api::notes::list(api::notes::Filter::default()).await }
    });

    let row_order_res = use_resource(move || async move { api::row_order::get(tile_id).await });
    let mut row_order = use_signal(Vec::<String>::new);
    use_effect(move || {
        if let Some(Ok(r)) = row_order_res.read().as_ref() {
            row_order.set(r.order.clone());
        }
    });

    // UI state
    let mut sort_key = use_signal(|| None::<String>);
    let mut sort_dir = use_signal(|| "asc".to_string());
    let mut sort_mode = use_signal(|| "column".to_string());
    let mut hidden = use_signal(|| {
        platform::get_list(&hidden_key(tile_id)).unwrap_or_else(|| vec!["url".to_string()])
    });
    let mut show_col_picker = use_signal(|| false);
    let mut show_filters = use_signal(|| false);
    let mut filters = use_signal(HashMap::<String, Vec<String>>::new);
    let mut open_filter_col = use_signal(|| None::<String>);
    let mut filter_anchor = use_signal(|| (0.0_f64, 0.0_f64));
    let filter_search = use_signal(String::new);
    let hovered_row = use_signal(|| None::<usize>);
    let drag_row = use_signal(|| None::<String>);
    let drag_over_idx = use_signal(|| None::<usize>);
    let context_menu = use_signal(|| None::<(f64, f64, String)>);
    let note_init = use_signal(|| None::<CreateNoteInput>);
    let note_editing = use_signal(|| None::<Note>);
    let detail_item = use_signal(|| None::<(String, String, i64, String)>);

    let commands = cmds_sig.read().clone();

    // ----- read fetched data -----
    let gh_read = gh.read();
    let loading = gh_read.is_none();
    let results: Vec<(String, Result<Value, String>)> = gh_read.clone().unwrap_or_default();
    drop(gh_read);

    if loading {
        return rsx! {
            div { class: "loading-text", "Running gh command…" }
        };
    }

    let errors: Vec<String> = results
        .iter()
        .filter_map(|(_, r)| r.as_ref().err().cloned())
        .collect();

    let mut raw: Vec<Value> = Vec::new();
    for (cmd, res) in &results {
        if let Ok(output) = res {
            let rows = if output.is_array() {
                output.as_array().cloned().unwrap_or_default()
            } else {
                vec![output.clone()]
            };
            let repo_from_cmd = parse_repo_from_command(cmd);
            for mut row in rows {
                if let Some(rc) = &repo_from_cmd {
                    let missing = row.get("repository").map(|v| v.is_null()).unwrap_or(true);
                    if missing {
                        let name = rc.split('/').nth(1).unwrap_or(rc);
                        if let Some(o) = row.as_object_mut() {
                            o.insert(
                                "repository".into(),
                                json!({"nameWithOwner": rc, "name": name}),
                            );
                        }
                    }
                }
                raw.push(row);
            }
        }
    }

    if !errors.is_empty() && raw.is_empty() {
        return rsx! {
            div {
                for (i , e) in errors.iter().enumerate() {
                    div { key: "{i}", class: "error-text", "{e}" }
                }
            }
        };
    }
    if raw.is_empty() {
        return rsx! {
            div { class: "muted text-xs", "No results" }
        };
    }

    // notes key map
    let note_map: HashMap<String, Note> = match all_notes.read().as_ref() {
        Some(Ok(notes)) => notes
            .iter()
            .filter_map(|n| {
                Some((
                    format!(
                        "{}|{}|{}",
                        n.repo.clone()?,
                        n.ref_type.clone()?,
                        n.ref_number?
                    ),
                    n.clone(),
                ))
            })
            .collect(),
        _ => HashMap::new(),
    };

    // merged extractors
    let mut extractors = state.global_extractors();
    if let Some(fe) = &config.field_extractors {
        extractors.extend(fe.clone());
    }

    // keys
    let all_keys: Vec<String> = if let Some(cols) = &config.columns {
        cols.clone()
    } else {
        let mut seen = HashSet::new();
        let mut keys = Vec::new();
        for row in &raw {
            if let Some(o) = row.as_object() {
                for k in o.keys() {
                    if seen.insert(k.clone()) {
                        keys.push(k.clone());
                    }
                }
            }
        }
        keys
    };
    let hidden_set: HashSet<String> = hidden.read().iter().cloned().collect();
    let keys: Vec<String> = all_keys
        .iter()
        .filter(|k| !hidden_set.contains(*k))
        .cloned()
        .collect();

    // sorted
    let mode = sort_mode.read().clone();
    let skey = sort_key.read().clone();
    let sdir = sort_dir.read().clone();
    let order = row_order.read().clone();
    let sorted: Vec<Value> = if mode == "priority" {
        let key_to_row: HashMap<String, Value> = raw
            .iter()
            .filter_map(|r| row_key(r).map(|k| (k, r.clone())))
            .collect();
        let positioned: HashSet<&String> = order.iter().collect();
        let mut out: Vec<Value> = order
            .iter()
            .filter_map(|k| key_to_row.get(k).cloned())
            .collect();
        for r in &raw {
            if !row_key(r).map(|k| positioned.contains(&k)).unwrap_or(false) {
                out.push(r.clone());
            }
        }
        out
    } else if let Some(sk) = &skey {
        let mut v = raw.clone();
        v.sort_by(|a, b| {
            let av = extract_display(a.get(sk).unwrap_or(&Value::Null), ex_of(&extractors, sk));
            let bv = extract_display(b.get(sk).unwrap_or(&Value::Null), ex_of(&extractors, sk));
            let c = cmp_vals(&av, &bv);
            if sdir == "asc" {
                c
            } else {
                c.reverse()
            }
        });
        v
    } else {
        raw.clone()
    };

    // unique column values for filter dropdowns
    let mut column_values: HashMap<String, Vec<String>> = HashMap::new();
    for k in &all_keys {
        let mut seen = HashSet::new();
        let mut vals = Vec::new();
        for row in &sorted {
            let d = extract_display(row.get(k).unwrap_or(&Value::Null), ex_of(&extractors, k));
            if seen.insert(d.clone()) {
                vals.push(d);
            }
        }
        vals.sort_by(|a, b| cmp_vals(a, b));
        column_values.insert(k.clone(), vals);
    }

    // filters
    let filters_now = filters.read().clone();
    let active_filter_count = filters_now.values().filter(|v| !v.is_empty()).count();
    let rows: Vec<Value> = if active_filter_count == 0 {
        sorted
    } else {
        sorted
            .into_iter()
            .filter(|row| {
                filters_now.iter().all(|(col, vals)| {
                    vals.is_empty()
                        || vals.contains(&extract_display(
                            row.get(col).unwrap_or(&Value::Null),
                            ex_of(&extractors, col),
                        ))
                })
            })
            .collect()
    };

    // Ordered keys of the currently displayed rows (for drag-reorder math).
    let display_keys: Vec<String> = rows.iter().filter_map(row_key).collect();

    // ----- handlers (closures capture Copy signals → Copy, 'static) -----
    let mut handle_sort = move |k: String| {
        sort_mode.set("column".into());
        if sort_key.read().as_deref() == Some(k.as_str()) {
            let d = if sort_dir.read().as_str() == "asc" {
                "desc"
            } else {
                "asc"
            };
            sort_dir.set(d.into());
        } else {
            sort_key.set(Some(k));
            sort_dir.set("asc".into());
        }
    };
    let mut toggle_column = move |k: String| {
        let mut h = hidden.write();
        if let Some(pos) = h.iter().position(|x| *x == k) {
            h.remove(pos);
        } else {
            h.push(k);
        }
        platform::set_list(&hidden_key(tile_id), &h);
    };

    let cur_sort_key = sort_key.read().clone();
    let cur_mode = sort_mode.read().clone();
    let cur_dir = sort_dir.read().clone();
    let open_col = open_filter_col.read().clone();

    rsx! {
        div { style: "height:100%; display:flex; flex-direction:column; gap:4px;",
            div { class: "tile-toolbar",
                if !errors.is_empty() {
                    span { class: "text-xs", style: "color:var(--warn); margin-right:auto;", "{errors.len()} command(s) failed" }
                }
                if active_filter_count > 0 {
                    button { class: "chip warn", onclick: move |_| filters.set(HashMap::new()),
                        Icon { width: 12, height: 12, icon: LdX }
                        "Clear filters ({active_filter_count})"
                    }
                }
                button {
                    class: if show_filters() || active_filter_count > 0 { "chip active" } else { "chip" },
                    onclick: move |_| { let v = !show_filters(); show_filters.set(v); },
                    Icon { width: 12, height: 12, icon: LdFilter }
                    if active_filter_count > 0 { "Filters ({active_filter_count})" } else { "Filters" }
                }
                button {
                    class: if cur_mode == "priority" { "chip active" } else { "chip" },
                    onclick: move |_| {
                        if sort_mode.read().as_str() == "priority" {
                            sort_mode.set("column".into());
                            sort_key.set(None);
                        } else {
                            sort_mode.set("priority".into());
                        }
                    },
                    Icon { width: 12, height: 12, icon: LdListOrdered }
                    "Priority"
                }
                button {
                    class: if show_col_picker() { "chip active" } else { "chip" },
                    onclick: move |_| { let v = !show_col_picker(); show_col_picker.set(v); },
                    Icon { width: 12, height: 12, icon: LdColumns2 }
                    "Columns"
                }
            }

            if show_col_picker() {
                div { style: "position:fixed; inset:0; z-index:30;", onclick: move |_| show_col_picker.set(false) }
                div {
                    class: "popover popover-menu",
                    style: "position:absolute; right:12px; top:40px; max-height:280px; overflow-y:auto; z-index:40;",
                    for k in all_keys.clone() {
                        label { key: "{k}", class: "checkbox-row",
                            input {
                                r#type: "checkbox",
                                checked: !hidden_set.contains(&k),
                                onclick: move |_| toggle_column(k.clone()),
                            }
                            span { "{k}" }
                        }
                    }
                }
            }

            div { style: "overflow:auto; flex:1;",
                table { class: "gh-table",
                    thead {
                        tr {
                            th { class: "num", "#" }
                            for k in keys.clone() {
                                th { key: "{k}",
                                    span { style: "display:inline-flex; align-items:center; gap:4px; width:100%;",
                                        span {
                                            class: "col-head",
                                            onclick: {
                                                let k = k.clone();
                                                move |_| handle_sort(k.clone())
                                            },
                                            "{k}"
                                            if cur_mode == "column" && cur_sort_key.as_deref() == Some(k.as_str()) {
                                                if cur_dir == "asc" {
                                                    Icon { width: 12, height: 12, icon: LdChevronUp }
                                                } else {
                                                    Icon { width: 12, height: 12, icon: LdChevronDown }
                                                }
                                            } else {
                                                Icon { width: 12, height: 12, icon: LdChevronsUpDown }
                                            }
                                        }
                                        if show_filters() {
                                            button {
                                                style: "margin-left:auto; background:none; border:none; color:var(--muted); cursor:pointer;",
                                                onclick: {
                                                    let k = k.clone();
                                                    move |e: Event<MouseData>| {
                                                        e.stop_propagation();
                                                        if open_filter_col.read().as_deref() == Some(k.as_str()) {
                                                            open_filter_col.set(None);
                                                        } else {
                                                            let c = e.client_coordinates();
                                                            filter_anchor.set((c.x, c.y));
                                                            open_filter_col.set(Some(k.clone()));
                                                        }
                                                    }
                                                },
                                                Icon { width: 12, height: 12, icon: LdFilter }
                                                if filters_now.get(&k).map(|v| v.len()).unwrap_or(0) > 0 {
                                                    span { style: "margin-left:2px;", "{filters_now.get(&k).map(|v| v.len()).unwrap_or(0)}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            th { style: "width:24px;" }
                        }
                    }
                    tbody {
                        for (i , row) in rows.iter().enumerate() {
                            {render_row(
                                i, row, &keys, &commands, &extractors, &filters_now, &note_map,
                                hovered_row, filters, note_init, note_editing, detail_item, context_menu,
                                display_keys.clone(), row_order, sort_mode, tile_id, drag_row, drag_over_idx,
                            )}
                        }
                        if rows.is_empty() {
                            tr {
                                td {
                                    colspan: "{keys.len() + 2}",
                                    style: "text-align:center; padding:24px; color:var(--muted);",
                                    "No rows match the current filters"
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(col) = open_col {
            {render_filter_dropdown(
                col.clone(),
                filter_anchor(),
                column_values.get(&col).cloned().unwrap_or_default(),
                filters_now.get(&col).cloned().unwrap_or_default(),
                filter_search,
                filters,
                open_filter_col,
            )}
        }

        if let Some((x , y , key)) = context_menu() {
            {render_context_menu(x, y, key, row_order, sort_mode, tile_id, context_menu)}
        }

        if let Some(init) = note_init() {
            {render_note_create(init, note_init, state)}
        }
        if let Some(note) = note_editing() {
            {render_note_edit(note, note_editing, state)}
        }
        if let Some((repo , ref_type , number , url)) = detail_item() {
            DetailPanel {
                repo,
                ref_type,
                number,
                url,
                on_close: move |_| { let mut d = detail_item; d.set(None); },
            }
        }
    }
}

// ---------- row + cell rendering (signals are Copy; no borrows captured in closures) ----------

/// Reorder rows by priority and persist the new order server-side (the same
/// store the backup function serializes). Mirrors React's `reorderRows`.
fn reorder_rows(
    from_key: &str,
    to_idx: usize,
    display_keys: &[String],
    mut row_order: Signal<Vec<String>>,
    mut sort_mode: Signal<String>,
    tile_id: i64,
) {
    let Some(from_idx) = display_keys.iter().position(|k| k == from_key) else {
        return;
    };
    let mut new_display: Vec<String> = display_keys.to_vec();
    let item = new_display.remove(from_idx);
    let to = to_idx.min(new_display.len());
    new_display.insert(to, item);

    let displayed: HashSet<&String> = display_keys.iter().collect();
    let non_displayed: Vec<String> = row_order
        .peek()
        .iter()
        .filter(|k| !displayed.contains(k))
        .cloned()
        .collect();
    let mut next = new_display;
    next.extend(non_displayed);

    row_order.set(next.clone());
    sort_mode.set("priority".into());
    spawn(async move {
        let _ = api::row_order::set(tile_id, next).await;
    });
}

#[allow(clippy::too_many_arguments)]
fn render_row(
    index: usize,
    row: &Value,
    keys: &[String],
    commands: &[String],
    extractors: &BTreeMap<String, String>,
    filters_now: &HashMap<String, Vec<String>>,
    note_map: &HashMap<String, Note>,
    mut hovered_row: Signal<Option<usize>>,
    filters: Signal<HashMap<String, Vec<String>>>,
    mut note_init: Signal<Option<CreateNoteInput>>,
    mut note_editing: Signal<Option<Note>>,
    detail_item: Signal<Option<(String, String, i64, String)>>,
    mut context_menu: Signal<Option<(f64, f64, String)>>,
    display_keys: Vec<String>,
    row_order: Signal<Vec<String>>,
    sort_mode: Signal<String>,
    tile_id: i64,
    mut drag_row: Signal<Option<String>>,
    mut drag_over_idx: Signal<Option<usize>>,
) -> Element {
    let r_key = row_key(row);
    let url = row.get("url").and_then(|v| v.as_str()).map(String::from);
    let repo = row_repo(row);
    let ref_type = row_ref_type(row, commands).to_string();
    let ref_num = row_number(row);
    let is_hovered = *hovered_row.read() == Some(index);
    let is_drag_over = *drag_over_idx.read() == Some(index);
    let draggable = r_key.is_some();

    let has_note = match (&repo, ref_num) {
        (Some(r), Some(n)) => note_map.contains_key(&format!("{r}|{ref_type}|{n}")),
        _ => false,
    };
    let existing_note = match (&repo, ref_num) {
        (Some(r), Some(n)) => note_map.get(&format!("{r}|{ref_type}|{n}")).cloned(),
        _ => None,
    };
    let prefill = CreateNoteInput {
        title: row
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        repo: repo.clone(),
        ref_type: Some(ref_type.clone()),
        ref_number: ref_num,
        ..Default::default()
    };

    let r_key_ctx = r_key.clone();
    let r_key_ds = r_key.clone();
    let r_key_drop = r_key.clone();
    let dk = display_keys;

    rsx! {
        tr {
            class: if is_drag_over { "drag-over" } else { "" },
            draggable,
            onmouseenter: move |_| hovered_row.set(Some(index)),
            onmouseleave: move |_| hovered_row.set(None),
            oncontextmenu: move |e: Event<MouseData>| {
                if let Some(k) = r_key_ctx.clone() {
                    e.prevent_default();
                    let c = e.client_coordinates();
                    context_menu.set(Some((c.x, c.y, k)));
                }
            },
            ondragstart: move |_| {
                if let Some(k) = &r_key_ds {
                    drag_row.set(Some(k.clone()));
                }
            },
            ondragover: move |e: Event<DragData>| {
                e.prevent_default();
                drag_over_idx.set(Some(index));
            },
            ondrop: move |e: Event<DragData>| {
                e.prevent_default();
                drag_over_idx.set(None);
                let from = drag_row.read().clone();
                drag_row.set(None);
                if let Some(from) = from {
                    if Some(from.as_str()) != r_key_drop.as_deref() {
                        reorder_rows(&from, index, &dk, row_order, sort_mode, tile_id);
                    }
                }
            },
            ondragend: move |_| {
                drag_row.set(None);
                drag_over_idx.set(None);
            },
            td { class: "num-cell",
                if is_hovered && draggable {
                    Icon { width: 14, height: 14, icon: LdGripVertical }
                } else {
                    "{index + 1}"
                }
            }
            for k in keys.iter() {
                {render_cell(
                    k.clone(), row, url.clone(), repo.clone(), ref_type.clone(), ref_num,
                    extractors, filters_now, filters, detail_item,
                )}
            }
            td { style: "width:24px; padding:4px;",
                if has_note || is_hovered {
                    button {
                        class: "icon-btn",
                        style: if has_note { "color:var(--warn);" } else { "" },
                        title: "Add private note",
                        onclick: move |_| {
                            if let Some(n) = existing_note.clone() {
                                note_editing.set(Some(n));
                            } else {
                                note_init.set(Some(prefill.clone()));
                            }
                        },
                        Icon { width: 14, height: 14, icon: LdStickyNote }
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_cell(
    k: String,
    row: &Value,
    url: Option<String>,
    repo: Option<String>,
    ref_type: String,
    ref_num: Option<i64>,
    extractors: &BTreeMap<String, String>,
    filters_now: &HashMap<String, Vec<String>>,
    mut filters: Signal<HashMap<String, Vec<String>>>,
    mut detail_item: Signal<Option<(String, String, i64, String)>>,
) -> Element {
    let ex = ex_of(extractors, &k);
    let val = row.get(&k).cloned().unwrap_or(Value::Null);
    let display = extract_display(&val, ex);
    let is_dt = val.as_str().map(is_datetime_string).unwrap_or(false);
    let cell_display = if is_dt {
        format_date_only(val.as_str().unwrap_or(""))
    } else {
        display.clone()
    };
    let tooltip = if is_dt {
        val.as_str().unwrap_or("").to_string()
    } else if ex.is_some() && val.is_object() {
        serde_json::to_string_pretty(&val).unwrap_or_default()
    } else {
        display.clone()
    };
    let is_active = filters_now
        .get(&k)
        .map(|v| v.contains(&display))
        .unwrap_or(false);

    if k == "number" {
        if let Some(u) = &url {
            return rsx! {
                td {
                    a {
                        href: "{u}",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        style: "font-variant-numeric:tabular-nums;",
                        "{cell_display}"
                    }
                }
            };
        }
    }
    if k == "title" {
        if let (Some(u), Some(r), Some(n)) = (url.clone(), repo.clone(), ref_num) {
            let rt = ref_type.clone();
            return rsx! {
                td {
                    Tooltip { text: tooltip,
                        span {
                            class: "cell-clickable",
                            onclick: move |_| detail_item.set(Some((r.clone(), rt.clone(), n, u.clone()))),
                            "{cell_display}"
                        }
                    }
                }
            };
        }
    }

    let kc = k.clone();
    let dc = display.clone();
    rsx! {
        td {
            Tooltip { text: tooltip,
                span {
                    class: if is_active { "cell-clickable cell-active" } else { "cell-clickable" },
                    onclick: move |_| {
                        if !dc.is_empty() {
                            let mut f = filters.write();
                            let e = f.entry(kc.clone()).or_default();
                            if let Some(p) = e.iter().position(|v| *v == dc) {
                                e.remove(p);
                            } else {
                                e.push(dc.clone());
                            }
                        }
                    },
                    "{cell_display}"
                }
            }
        }
    }
}

fn render_filter_dropdown(
    col: String,
    anchor: (f64, f64),
    values: Vec<String>,
    selected: Vec<String>,
    mut search: Signal<String>,
    mut filters: Signal<HashMap<String, Vec<String>>>,
    mut open_filter_col: Signal<Option<String>>,
) -> Element {
    let (x, y) = anchor;
    let q = search.read().to_lowercase();
    let filtered: Vec<String> = values
        .iter()
        .filter(|v| v.to_lowercase().contains(&q))
        .cloned()
        .collect();
    let all_values = values.clone();
    let col_all = col.clone();
    let col_clear = col.clone();

    rsx! {
        div { style: "position:fixed; inset:0; z-index:9998;", onclick: move |_| open_filter_col.set(None) }
        div {
            class: "popover",
            style: "left:{x}px; top:{y + 16.0}px; min-width:180px; max-width:280px; max-height:256px; z-index:9999;",
            div { style: "display:flex; align-items:center; gap:4px; padding:8px 8px 4px;",
                Icon { width: 12, height: 12, icon: LdSearch }
                input {
                    autofocus: true,
                    style: "flex:1; background:none; border:none; color:var(--text); outline:none; font-size:12px;",
                    placeholder: "Filter {col}…",
                    value: "{search}",
                    oninput: move |e| search.set(e.value()),
                }
            }
            div { style: "display:flex; gap:8px; padding:0 8px 4px; border-bottom:1px solid var(--border);",
                button {
                    style: "background:none; border:none; color:var(--accent); font-size:12px; cursor:pointer;",
                    onclick: move |_| { filters.write().insert(col_all.clone(), all_values.clone()); },
                    "All"
                }
                button {
                    style: "background:none; border:none; color:var(--muted); font-size:12px; cursor:pointer;",
                    onclick: move |_| { filters.write().insert(col_clear.clone(), Vec::new()); },
                    "Clear"
                }
                if !selected.is_empty() {
                    span { style: "margin-left:auto; color:var(--muted-dim); font-size:12px;", "{selected.len()} selected" }
                }
            }
            div { style: "overflow-y:auto; flex:1; padding:4px 0;",
                if filtered.is_empty() {
                    div { class: "muted text-xs", style: "padding:8px 12px;", "No values" }
                }
                for val in filtered {
                    label { key: "{val}", class: "checkbox-row",
                        input {
                            r#type: "checkbox",
                            checked: selected.contains(&val),
                            onclick: {
                                let col = col.clone();
                                let val = val.clone();
                                move |_| {
                                    let mut f = filters.write();
                                    let e = f.entry(col.clone()).or_default();
                                    if let Some(p) = e.iter().position(|v| *v == val) {
                                        e.remove(p);
                                    } else {
                                        e.push(val.clone());
                                    }
                                }
                            },
                        }
                        span { style: "overflow:hidden; text-overflow:ellipsis; white-space:nowrap;",
                            if val.is_empty() { "(empty)" } else { "{val}" }
                        }
                    }
                }
            }
        }
    }
}

fn render_context_menu(
    x: f64,
    y: f64,
    key: String,
    mut row_order: Signal<Vec<String>>,
    mut sort_mode: Signal<String>,
    tile_id: i64,
    mut context_menu: Signal<Option<(f64, f64, String)>>,
) -> Element {
    let set_order = move |next: Vec<String>| {
        row_order.set(next.clone());
        sort_mode.set("priority".into());
        spawn(async move {
            let _ = api::row_order::set(tile_id, next).await;
        });
        context_menu.set(None);
    };
    let key_top = key.clone();
    let key_bottom = key.clone();

    rsx! {
        div { style: "position:fixed; inset:0; z-index:9998;", onclick: move |_| context_menu.set(None) }
        div { class: "popover popover-menu", style: "left:{x}px; top:{y}px; z-index:9999;",
            button {
                class: "popover-item",
                onclick: move |_| {
                    let mut next: Vec<String> = row_order.read().iter().filter(|u| **u != key_top).cloned().collect();
                    next.insert(0, key_top.clone());
                    let mut set_order = set_order;
                    set_order(next);
                },
                "Send to Top"
            }
            button {
                class: "popover-item",
                onclick: move |_| {
                    let mut next: Vec<String> = row_order.read().iter().filter(|u| **u != key_bottom).cloned().collect();
                    next.push(key_bottom.clone());
                    let mut set_order = set_order;
                    set_order(next);
                },
                "Send to Bottom"
            }
        }
    }
}

fn render_note_create(
    init: CreateNoteInput,
    mut note_init: Signal<Option<CreateNoteInput>>,
    state: crate::state::AppState,
) -> Element {
    rsx! {
        NoteDialog {
            initial: init,
            on_confirm: move |input: CreateNoteInput| {
                spawn(async move {
                    if api::notes::create(&input).await.is_ok() {
                        state.invalidate_notes();
                    }
                });
                note_init.set(None);
            },
            on_close: move |_| note_init.set(None),
        }
    }
}

fn render_note_edit(
    note: Note,
    mut note_editing: Signal<Option<Note>>,
    state: crate::state::AppState,
) -> Element {
    let note_id = note.id;
    rsx! {
        NoteDialog {
            note,
            on_confirm: move |input: CreateNoteInput| {
                spawn(async move {
                    if api::notes::update(note_id, &input).await.is_ok() {
                        state.invalidate_notes();
                    }
                });
                note_editing.set(None);
            },
            on_close: move |_| note_editing.set(None),
        }
    }
}
