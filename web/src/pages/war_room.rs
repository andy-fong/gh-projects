use std::collections::{BTreeMap, HashMap, HashSet};

use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{
    LdChevronDown, LdChevronUp, LdClipboardPaste, LdCopy, LdExternalLink, LdLink, LdMessageSquare,
    LdPencil, LdPlus, LdStickyNote, LdTrash2,
};
use dioxus_free_icons::Icon;

use crate::api;
use crate::components::common::Prose;
use crate::components::dialogs::slack_update_dialog::SlackUpdateDialog;
use crate::components::dialogs::war_room_group_dialog::WarRoomGroupDialog;
use crate::components::dialogs::war_room_item_dialog::WarRoomItemDialog;
use crate::components::dialogs::war_room_link_dialog::WarRoomLinkDialog;
use crate::state::{use_app_state, AppState};
use crate::types::{
    CreateGroupInput, CreateItemInput, GroupWithItems, Repo, UpdateGroupInput, UpdateItemInput,
    UpdateWarRoomInput, WarRoomGroup, WarRoomItem,
};
use crate::war_room::{group_rollup_stage, github_url, stage_badge_class, stage_label};
use crate::Route;

/// Resolution map: item id -> (item, owning group's repo, owning group's name).
type ItemIndex = HashMap<i64, (WarRoomItem, Option<String>, String)>;

/// A mirror row's displayed stage/label come from its source item.
fn effective_stage(item: &WarRoomItem, index: &ItemIndex) -> String {
    match item.source_item_id.and_then(|sid| index.get(&sid)) {
        Some((src, _, _)) => src.stage.clone(),
        None => item.stage.clone(),
    }
}

/// Label for the Slack update. A mirror row is prefixed with its source group
/// (e.g. "Envoy Releases / v1.37.4") so linked items are unambiguous.
fn slack_item_label(item: &WarRoomItem, index: &ItemIndex) -> String {
    match item.source_item_id.and_then(|sid| index.get(&sid)) {
        Some((src, _, gname)) => format!("{} / {}", gname, src.label),
        None => item.label.clone(),
    }
}

/// Split a group name into `(product, Some(version))` when it ends in a version
/// token (e.g. "kgateway OSS v2.3.3" → ("kgateway OSS", "v2.3.3")). Groups with
/// no trailing version (e.g. "Envoy Releases") return `(name, None)`.
fn split_product_version(name: &str) -> (String, Option<String>) {
    let trimmed = name.trim();
    if let Some(pos) = trimmed.rfind(char::is_whitespace) {
        let last = trimmed[pos + 1..].trim();
        let looks_versioned = last
            .strip_prefix('v')
            .unwrap_or(last)
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit());
        if looks_versioned {
            return (trimmed[..pos].trim().to_string(), Some(last.to_string()));
        }
    }
    (trimmed.to_string(), None)
}

/// Build a Slack-ready status update. Groups sharing a product prefix are nested
/// under one header; each release is a bullet whose emoji is the group's rollup
/// status, with its items as an indented sub-list. Groups with no version suffix
/// list their items directly. When `include_details` is false, the per-release
/// item sub-list is omitted (product + version headlines only).
fn generate_slack_update(
    room_name: &str,
    groups: &[GroupWithItems],
    index: &ItemIndex,
    emojis: &BTreeMap<String, String>,
    include_details: bool,
) -> String {
    let em = |stage: &str| emojis.get(stage).cloned().unwrap_or_default();
    let mut out = format!("Here is the latest status on {room_name}:\n");
    let mut last_header: Option<String> = None;

    for gi in groups {
        let stages: Vec<String> = gi.items.iter().map(|i| effective_stage(i, index)).collect();
        let (product, version) = split_product_version(&gi.group.name);

        match version {
            Some(ver) => {
                if last_header.as_deref() != Some(product.as_str()) {
                    out.push_str(&format!("\n{product}:\n"));
                    last_header = Some(product.clone());
                }
                let emoji = group_rollup_stage(&stages).map(|s| em(&s)).unwrap_or_default();
                out.push_str(&format!("- {emoji} {ver}\n"));
                // Items as an indented second-level list (skipped in summary mode).
                if include_details {
                    for i in &gi.items {
                        let e = em(&effective_stage(i, index));
                        let lbl = slack_item_label(i, index);
                        let note = i.note.trim();
                        if note.is_empty() {
                            out.push_str(&format!("    - {e} {lbl}\n"));
                        } else {
                            out.push_str(&format!("    - {e} {lbl} ({note})\n"));
                        }
                    }
                }
            }
            None => {
                out.push_str(&format!("\n{}:\n", gi.group.name));
                last_header = None;
                for i in &gi.items {
                    let emoji = em(&effective_stage(i, index));
                    let lbl = slack_item_label(i, index);
                    let note = i.note.trim();
                    if note.is_empty() {
                        out.push_str(&format!("- {emoji} {lbl}\n"));
                    } else {
                        out.push_str(&format!("- {emoji} {lbl} ({note})\n"));
                    }
                }
            }
        }
    }
    out
}

// ---- pure mappers (replace-style: carry the full record) ----

fn item_to_update(item: &WarRoomItem, position: Option<i64>) -> UpdateItemInput {
    UpdateItemInput {
        label: Some(item.label.clone()),
        ref_type: item.ref_type.clone(),
        ref_number: item.ref_number,
        note: Some(item.note.clone()),
        stage: Some(item.stage.clone()),
        checklist: Some(item.checklist_items()),
        depends_on: item.depends_on,
        position,
    }
}

/// Snapshot an item into a create payload — an independent copy (not a mirror).
fn item_to_create(item: &WarRoomItem) -> CreateItemInput {
    CreateItemInput {
        label: item.label.clone(),
        ref_type: item.ref_type.clone(),
        ref_number: item.ref_number,
        note: Some(item.note.clone()),
        stage: Some(item.stage.clone()),
        checklist: Some(item.checklist_items()),
        depends_on: item.depends_on,
        source_item_id: None,
    }
}

fn create_to_update_item(c: CreateItemInput) -> UpdateItemInput {
    UpdateItemInput {
        label: Some(c.label),
        ref_type: c.ref_type,
        ref_number: c.ref_number,
        note: c.note,
        stage: c.stage,
        checklist: c.checklist,
        depends_on: c.depends_on,
        position: None,
    }
}

fn group_to_update(g: &WarRoomGroup, position: Option<i64>) -> UpdateGroupInput {
    UpdateGroupInput {
        name: Some(g.name.clone()),
        repo_id: g.repo_id,
        repo: g.repo.clone(),
        position,
    }
}

fn create_to_update_group(c: CreateGroupInput, position: Option<i64>) -> UpdateGroupInput {
    UpdateGroupInput {
        name: Some(c.name),
        repo_id: c.repo_id,
        repo: c.repo,
        position,
    }
}

/// Dialog control shared with `GroupCard`.
#[derive(Clone, Copy)]
struct WarCtx {
    state: AppState,
    // Some(None) = add group; Some(Some(g)) = edit group g.
    group_dialog: Signal<Option<Option<WarRoomGroup>>>,
    // Some((group_id, maybe_item)) = add/edit item in a group.
    item_dialog: Signal<Option<(i64, Option<WarRoomItem>)>>,
    // Some(group_id) = open the "link an item" picker for that group.
    link_dialog: Signal<Option<i64>>,
    // An item placed on the clipboard, pasteable into any group as a copy.
    copied_item: Signal<Option<WarRoomItem>>,
    // id -> (item, owning group's repo, owning group's name); lets a mirror row
    // render its source's live data and resolves "depends on" badges.
    items_by_id: Signal<HashMap<i64, (WarRoomItem, Option<String>, String)>>,
}

#[component]
pub fn WarRoomPage(id: i64) -> Element {
    let state = use_app_state();

    let mut id_sig = use_signal(|| id);
    if *id_sig.peek() != id {
        id_sig.set(id);
    }

    let detail = use_resource(move || {
        let _ = state.war_rooms_ver.read();
        let id = id_sig();
        async move { api::war_rooms::get(id).await }
    });
    let repos_res = use_resource(move || {
        let _ = state.repos_ver.read();
        async move { api::repos::list().await }
    });

    let nav = use_navigator();
    let group_dialog = use_signal(|| None::<Option<WarRoomGroup>>);
    let item_dialog = use_signal(|| None::<(i64, Option<WarRoomItem>)>);
    let link_dialog = use_signal(|| None::<i64>);
    let copied_item = use_signal(|| None::<WarRoomItem>);
    let mut items_by_id =
        use_signal(HashMap::<i64, (WarRoomItem, Option<String>, String)>::new);
    let mut renaming = use_signal(|| false);
    let mut rename_value = use_signal(String::new);
    let mut confirm_delete = use_signal(|| false);
    let mut editing_note = use_signal(|| false);
    let mut note_value = use_signal(String::new);
    let mut show_slack = use_signal(|| false);

    use_context_provider(|| WarCtx {
        state,
        group_dialog,
        item_dialog,
        link_dialog,
        copied_item,
        items_by_id,
    });

    let repos: Vec<Repo> = match repos_res.read().as_ref() {
        Some(Ok(v)) => v.clone(),
        _ => Vec::new(),
    };

    let detail_val = match detail.read().as_ref() {
        Some(Ok(d)) => Some(d.clone()),
        _ => None,
    };

    let Some(detail_val) = detail_val else {
        return rsx! {
            div { class: "empty-hint", "Loading war room…" }
        };
    };

    let room = detail_val.war_room.clone();
    let groups = detail_val.groups.clone();
    let room_name = room.name.clone();
    let room_id = room.id;

    // Keep the id -> (item, group repo, group name) map current so mirror rows
    // can resolve and render their source's live data, and "depends on" badges
    // can show the dependency's title + status.
    let item_index: ItemIndex = groups
        .iter()
        .flat_map(|g| {
            g.items
                .iter()
                .map(|i| (i.id, (i.clone(), g.group.repo.clone(), g.group.name.clone())))
        })
        .collect();
    if *items_by_id.peek() != item_index {
        items_by_id.set(item_index.clone());
    }

    // depends-on options for the item dialog, shown as "Group / Title".
    // Mirror rows are excluded so each real item appears once.
    let siblings: Vec<(i64, String)> = groups
        .iter()
        .flat_map(|g| {
            let gname = g.group.name.clone();
            g.items
                .iter()
                .filter(|i| i.source_item_id.is_none())
                .map(move |i| (i.id, format!("{} / {}", gname, i.label)))
        })
        .collect();

    let commit_rename = {
        let current = room_name.clone();
        move || {
            let name = rename_value.read().trim().to_string();
            if !name.is_empty() && name != current {
                spawn(async move {
                    let _ = api::war_rooms::update(
                        room_id,
                        &UpdateWarRoomInput {
                            name: Some(name),
                            ..Default::default()
                        },
                    )
                    .await;
                    state.invalidate_war_rooms();
                });
            }
            renaming.set(false);
        }
    };

    let group_count = groups.len();

    // Precompute the "link an item" picker data (rsx control-flow blocks can't
    // hold bare `let` statements, so this is built here).
    let link_data: Option<(i64, Vec<(i64, String)>, HashMap<i64, String>)> =
        link_dialog.read().clone().map(|gid| {
            let already: HashSet<i64> = groups
                .iter()
                .find(|g| g.group.id == gid)
                .map(|g| g.items.iter().filter_map(|i| i.source_item_id).collect())
                .unwrap_or_default();
            let candidates: Vec<(i64, String)> = groups
                .iter()
                .filter(|g| g.group.id != gid)
                .flat_map(|g| {
                    let gname = g.group.name.clone();
                    g.items
                        .iter()
                        .filter(|i| i.source_item_id.is_none() && !already.contains(&i.id))
                        .map(move |i| (i.id, format!("{} / {}", gname, i.label)))
                })
                .collect();
            let label_for: HashMap<i64, String> = groups
                .iter()
                .flat_map(|g| g.items.iter().map(|i| (i.id, i.label.clone())))
                .collect();
            (gid, candidates, label_for)
        });

    rsx! {
        div { class: "page",
            div { class: "page-header",
                if renaming() {
                    input {
                        class: "input",
                        style: "max-width: 360px; font-size:18px; font-weight:600;",
                        autofocus: true,
                        value: "{rename_value}",
                        oninput: move |e| rename_value.set(e.value()),
                        onblur: {
                            let mut commit = commit_rename.clone();
                            move |_| commit()
                        },
                        onkeydown: {
                            let mut commit = commit_rename.clone();
                            move |e| {
                                if e.key() == Key::Enter {
                                    commit();
                                } else if e.key() == Key::Escape {
                                    renaming.set(false);
                                }
                            }
                        },
                    }
                } else {
                    h1 {
                        class: "page-title editable",
                        title: "Click to rename",
                        onclick: {
                            let name = room_name.clone();
                            move |_| {
                                rename_value.set(name.clone());
                                renaming.set(true);
                            }
                        },
                        "{room_name}"
                    }
                }
                div { class: "toolbar",
                    select {
                        class: "select",
                        style: "width: auto;",
                        value: "{room.status}",
                        onchange: move |e| {
                            let status = e.value();
                            spawn(async move {
                                let _ = api::war_rooms::update(
                                        room_id,
                                        &UpdateWarRoomInput {
                                            status: Some(status),
                                            ..Default::default()
                                        },
                                    )
                                    .await;
                                state.invalidate_war_rooms();
                            });
                        },
                        option { value: "active", "Active" }
                        option { value: "resolved", "Resolved" }
                        option { value: "archived", "Archived" }
                    }
                    if confirm_delete() {
                        button {
                            class: "btn btn-danger btn-sm",
                            onclick: move |_| {
                                let nav = nav.clone();
                                spawn(async move {
                                    if api::war_rooms::delete(room_id).await.is_ok() {
                                        state.invalidate_war_rooms();
                                        nav.push(Route::Home {});
                                    }
                                });
                                confirm_delete.set(false);
                            },
                            Icon { width: 16, height: 16, icon: LdTrash2 }
                            "Confirm delete"
                        }
                        button {
                            class: "btn btn-sm",
                            onclick: move |_| confirm_delete.set(false),
                            "Cancel"
                        }
                    } else {
                        button {
                            class: "btn btn-sm",
                            title: "Delete this war room",
                            onclick: move |_| confirm_delete.set(true),
                            Icon { width: 16, height: 16, icon: LdTrash2 }
                            "Delete"
                        }
                    }
                    button {
                        class: "btn btn-sm",
                        title: "Edit note",
                        onclick: {
                            let l = room.notes.clone();
                            move |_| {
                                note_value.set(l.clone());
                                editing_note.set(true);
                            }
                        },
                        Icon { width: 16, height: 16, icon: LdStickyNote }
                        "Note"
                    }
                    button {
                        class: "btn btn-sm",
                        title: "Generate a Slack status update",
                        onclick: move |_| show_slack.set(true),
                        Icon { width: 16, height: 16, icon: LdMessageSquare }
                        "Slack update"
                    }
                    button {
                        class: "btn btn-primary btn-sm",
                        onclick: move |_| group_dialog.clone().set(Some(None)),
                        Icon { width: 16, height: 16, icon: LdPlus }
                        "Add group"
                    }
                }
            }

            // Markdown note box — pinned at top when it has content.
            if editing_note() {
                div { class: "wr-notes-edit",
                    textarea {
                        class: "textarea mono",
                        rows: 6,
                        autofocus: true,
                        placeholder: "Note in Markdown — context, useful links, runbooks, dashboards…",
                        value: "{note_value}",
                        oninput: move |e| note_value.set(e.value()),
                    }
                    div { class: "form-actions",
                        button {
                            class: "btn btn-ghost btn-sm",
                            onclick: move |_| editing_note.set(false),
                            "Cancel"
                        }
                        button {
                            class: "btn btn-primary btn-sm",
                            onclick: move |_| {
                                let note = note_value.read().clone();
                                spawn(async move {
                                    let _ = api::war_rooms::update(
                                            room_id,
                                            &UpdateWarRoomInput {
                                                notes: Some(note),
                                                ..Default::default()
                                            },
                                        )
                                        .await;
                                    state.invalidate_war_rooms();
                                });
                                editing_note.set(false);
                            },
                            "Save"
                        }
                    }
                }
            } else if !room.notes.trim().is_empty() {
                div { class: "wr-notes",
                    div { class: "wr-notes-head",
                        span { class: "section-label",
                            Icon { width: 14, height: 14, icon: LdStickyNote }
                            "Note"
                        }
                        button {
                            class: "icon-btn",
                            title: "Edit note",
                            onclick: {
                                let l = room.notes.clone();
                                move |_| {
                                    note_value.set(l.clone());
                                    editing_note.set(true);
                                }
                            },
                            Icon { width: 14, height: 14, icon: LdPencil }
                        }
                    }
                    div { class: "wr-notes-body",
                        Prose { text: room.notes.clone(), blank_links: true }
                    }
                }
            }

            if groups.is_empty() {
                div { class: "empty-hint", style: "text-align:center; margin-top:3rem;",
                    "No groups yet. Add a group (pick a repo) to start tracking items."
                }
            } else {
                for (idx , g) in groups.iter().enumerate() {
                    GroupCard {
                        key: "{g.group.id}",
                        group: g.group.clone(),
                        items: g.items.clone(),
                        index: idx,
                        group_count,
                        all_groups: groups.clone(),
                    }
                }
            }
        }

        // ---- Group dialog ----
        if let Some(editing) = group_dialog.read().clone() {
            WarRoomGroupDialog {
                group: editing.clone(),
                repos: repos.clone(),
                on_confirm: move |input: CreateGroupInput| {
                    let editing = editing.clone();
                    spawn(async move {
                        if let Some(g) = editing {
                            let _ = api::war_rooms::update_group(
                                    g.id,
                                    &create_to_update_group(input, Some(g.position)),
                                )
                                .await;
                        } else {
                            let _ = api::war_rooms::create_group(room_id, &input).await;
                        }
                        state.invalidate_war_rooms();
                    });
                    group_dialog.clone().set(None);
                },
                on_close: move |_| group_dialog.clone().set(None),
            }
        }

        // ---- Item dialog ----
        if let Some((gid , editing)) = item_dialog.read().clone() {
            WarRoomItemDialog {
                item: editing.clone(),
                repo: groups.iter().find(|g| g.group.id == gid).and_then(|g| g.group.repo.clone()),
                siblings: siblings.clone(),
                on_confirm: move |input: CreateItemInput| {
                    let editing = editing.clone();
                    spawn(async move {
                        if let Some(it) = editing {
                            let _ = api::war_rooms::update_item(
                                    it.id,
                                    &UpdateItemInput {
                                        position: Some(it.position),
                                        ..create_to_update_item(input)
                                    },
                                )
                                .await;
                        } else {
                            let _ = api::war_rooms::create_item(gid, &input).await;
                        }
                        state.invalidate_war_rooms();
                    });
                    item_dialog.clone().set(None);
                },
                on_close: move |_| item_dialog.clone().set(None),
            }
        }

        // ---- Link (mirror existing item) dialog ----
        if let Some((gid , candidates , label_for)) = link_data.clone() {
            WarRoomLinkDialog {
                candidates,
                on_confirm: move |source_id: i64| {
                    let label = label_for.get(&source_id).cloned().unwrap_or_default();
                    spawn(async move {
                        let _ = api::war_rooms::create_item(
                                gid,
                                &CreateItemInput {
                                    label,
                                    source_item_id: Some(source_id),
                                    ..Default::default()
                                },
                            )
                            .await;
                        state.invalidate_war_rooms();
                    });
                    link_dialog.clone().set(None);
                },
                on_close: move |_| link_dialog.clone().set(None),
            }
        }

        // ---- Slack update dialog ----
        if show_slack() {
            SlackUpdateDialog {
                detailed: generate_slack_update(&room_name, &groups, &item_index, &state.stage_emojis(), true),
                summary: generate_slack_update(&room_name, &groups, &item_index, &state.stage_emojis(), false),
                on_close: move |_| show_slack.set(false),
            }
        }
    }
}

#[component]
fn GroupCard(
    group: WarRoomGroup,
    items: Vec<WarRoomItem>,
    index: usize,
    group_count: usize,
    all_groups: Vec<GroupWithItems>,
) -> Element {
    let ctx = use_context::<WarCtx>();
    let state = ctx.state;
    let group_id = group.id;

    // Reorder this group by swapping positions with a neighbour.
    let move_group = {
        let all_groups = all_groups.clone();
        let group = group.clone();
        move |delta: i32| {
            let target = index as i32 + delta;
            if target < 0 || target as usize >= all_groups.len() {
                return;
            }
            let other = all_groups[target as usize].group.clone();
            let a = group.clone();
            spawn(async move {
                let _ = api::war_rooms::update_group(a.id, &group_to_update(&a, Some(other.position)))
                    .await;
                let _ = api::war_rooms::update_group(other.id, &group_to_update(&other, Some(a.position)))
                    .await;
                state.invalidate_war_rooms();
            });
        }
    };
    let move_group_up = move_group.clone();
    let move_group_down = move_group.clone();

    let edit_group = {
        let group = group.clone();
        move |_| ctx.group_dialog.clone().set(Some(Some(group.clone())))
    };
    let delete_group = move |_| {
        spawn(async move {
            if api::war_rooms::delete_group(group_id).await.is_ok() {
                state.invalidate_war_rooms();
            }
        });
    };
    let add_item = move |_| ctx.item_dialog.clone().set(Some((group_id, None)));
    let link_item = move |_| ctx.link_dialog.clone().set(Some(group_id));

    // Paste the clipboard item into this group as an independent copy.
    let copied = ctx.copied_item.read().clone();
    let paste_item = {
        let copied = copied.clone();
        move |_| {
            let Some(ci) = copied.clone() else { return };
            spawn(async move {
                let _ = api::war_rooms::create_item(group_id, &item_to_create(&ci)).await;
                state.invalidate_war_rooms();
            });
        }
    };

    let repo_label = group.repo.clone().unwrap_or_default();
    let item_count = items.len();

    // Overall group status, rolled up from each item's effective stage (mirror
    // rows contribute their source item's stage).
    let stages: Vec<String> = items
        .iter()
        .map(|i| match i.source_item_id {
            Some(sid) => all_groups
                .iter()
                .flat_map(|g| g.items.iter())
                .find(|x| x.id == sid)
                .map(|x| x.stage.clone())
                .unwrap_or_else(|| i.stage.clone()),
            None => i.stage.clone(),
        })
        .collect();
    let rollup = group_rollup_stage(&stages);

    rsx! {
        div { class: "wr-group",
            div { class: "wr-group-head",
                div { class: "wr-group-title",
                    span { class: "wr-group-name", "{group.name}" }
                    if let Some(st) = rollup.clone() {
                        span {
                            class: "badge {stage_badge_class(&st)}",
                            title: "Overall group status",
                            "{stage_label(&st)}"
                        }
                    }
                    if !repo_label.is_empty() {
                        span { class: "label-chip", "{repo_label}" }
                    }
                }
                div { class: "wr-group-actions",
                    button {
                        class: "icon-btn",
                        disabled: index == 0,
                        title: "Move group up",
                        onclick: move |_| move_group_up(-1),
                        Icon { width: 15, height: 15, icon: LdChevronUp }
                    }
                    button {
                        class: "icon-btn",
                        disabled: index + 1 >= group_count,
                        title: "Move group down",
                        onclick: move |_| move_group_down(1),
                        Icon { width: 15, height: 15, icon: LdChevronDown }
                    }
                    button { class: "icon-btn", title: "Edit group", onclick: edit_group,
                        Icon { width: 15, height: 15, icon: LdPencil }
                    }
                    button { class: "icon-btn danger", title: "Delete group", onclick: delete_group,
                        Icon { width: 15, height: 15, icon: LdTrash2 }
                    }
                    if let Some(ci) = copied.clone() {
                        button { class: "btn btn-sm", title: "Paste \"{ci.label}\" here as a copy", onclick: paste_item,
                            Icon { width: 14, height: 14, icon: LdClipboardPaste }
                            "Paste"
                        }
                    }
                    button { class: "btn btn-sm", title: "Mirror an item from another group", onclick: link_item,
                        Icon { width: 14, height: 14, icon: LdLink }
                        "Link"
                    }
                    button { class: "btn btn-sm", onclick: add_item,
                        Icon { width: 14, height: 14, icon: LdPlus }
                        "Item"
                    }
                }
            }

            if items.is_empty() {
                div { class: "wr-empty", "No items." }
            } else {
                for (i_idx , item) in items.iter().enumerate() {
                    ItemRow {
                        key: "{item.id}",
                        item: item.clone(),
                        repo: group.repo.clone(),
                        index: i_idx,
                        item_count,
                        items: items.clone(),
                    }
                }
            }
        }
    }
}

#[component]
fn ItemRow(
    item: WarRoomItem,
    repo: Option<String>,
    index: usize,
    item_count: usize,
    items: Vec<WarRoomItem>,
) -> Element {
    let ctx = use_context::<WarCtx>();
    let state = ctx.state;
    let item_id = item.id;
    let is_ref = item.source_item_id.is_some();

    // A mirror row renders its source's live data (and the source's repo for
    // links); a normal row renders itself.
    let (disp, disp_repo, source_group) = match item.source_item_id {
        Some(sid) => match ctx.items_by_id.read().get(&sid) {
            Some((src, src_repo, src_name)) => {
                (src.clone(), src_repo.clone(), Some(src_name.clone()))
            }
            None => (item.clone(), repo.clone(), Some("removed".to_string())),
        },
        None => (item.clone(), repo.clone(), None),
    };

    let checklist = disp.checklist_items();
    let done_count = checklist.iter().filter(|c| c.done).count();

    let link = match (&disp_repo, disp.ref_number) {
        (Some(r), Some(n)) if !r.is_empty() => {
            Some(github_url(r, disp.ref_type.as_deref(), n))
        }
        _ => None,
    };

    let dep_label = disp.depends_on.and_then(|d| {
        ctx.items_by_id
            .read()
            .get(&d)
            .map(|(it, _, _)| format!("{} ({})", it.label, stage_label(&it.stage)))
    });

    // Reorder within the group.
    let move_item = {
        let items = items.clone();
        let item = item.clone();
        move |delta: i32| {
            let target = index as i32 + delta;
            if target < 0 || target as usize >= items.len() {
                return;
            }
            let other = items[target as usize].clone();
            let a = item.clone();
            spawn(async move {
                let _ = api::war_rooms::update_item(a.id, &item_to_update(&a, Some(other.position)))
                    .await;
                let _ = api::war_rooms::update_item(other.id, &item_to_update(&other, Some(a.position)))
                    .await;
                state.invalidate_war_rooms();
            });
        }
    };
    let move_up = move_item.clone();
    let move_down = move_item.clone();

    let edit_item = {
        let item = item.clone();
        move |_| ctx.item_dialog.clone().set(Some((item.group_id, Some(item.clone()))))
    };
    let delete_item = move |_| {
        spawn(async move {
            if api::war_rooms::delete_item(item_id).await.is_ok() {
                state.invalidate_war_rooms();
            }
        });
    };

    let ref_badge = disp.ref_number.map(|n| {
        let prefix = if disp.ref_type.as_deref() == Some("issue") { "#" } else { "PR #" };
        format!("{prefix}{n}")
    });

    rsx! {
        div { class: if is_ref { "wr-item wr-item-linked" } else { "wr-item" },
            div { class: "wr-item-main",
                span { class: "badge {stage_badge_class(&disp.stage)}", "{stage_label(&disp.stage)}" }
                span { class: "wr-item-label", "{disp.label}" }
                if let Some(src) = source_group.clone() {
                    span { class: "wr-link-chip", title: "Mirrors an item in \"{src}\" — edit the original to update",
                        Icon { width: 11, height: 11, icon: LdLink }
                        "{src}"
                    }
                }
                if let Some(rb) = ref_badge {
                    if let Some(href) = link.clone() {
                        a {
                            class: "wr-ref-link",
                            href: "{href}",
                            target: "_blank",
                            rel: "noreferrer",
                            "{rb}"
                            Icon { width: 12, height: 12, icon: LdExternalLink }
                        }
                    } else {
                        span { class: "label-chip", "{rb}" }
                    }
                }
                if let Some(dep) = dep_label {
                    span { class: "wr-dep", "depends on {dep}" }
                }
                if !checklist.is_empty() {
                    span { class: "wr-progress", "{done_count}/{checklist.len()}" }
                }
                div { class: "spacer" }
                button {
                    class: "icon-btn",
                    disabled: index == 0,
                    title: "Move up",
                    onclick: move |_| move_up(-1),
                    Icon { width: 14, height: 14, icon: LdChevronUp }
                }
                button {
                    class: "icon-btn",
                    disabled: index + 1 >= item_count,
                    title: "Move down",
                    onclick: move |_| move_down(1),
                    Icon { width: 14, height: 14, icon: LdChevronDown }
                }
                if !is_ref {
                    button {
                        class: "icon-btn",
                        title: "Copy item (paste into any group)",
                        onclick: {
                            let item = item.clone();
                            move |_| ctx.copied_item.clone().set(Some(item.clone()))
                        },
                        Icon { width: 14, height: 14, icon: LdCopy }
                    }
                    button { class: "icon-btn", title: "Edit item", onclick: edit_item,
                        Icon { width: 14, height: 14, icon: LdPencil }
                    }
                }
                button {
                    class: "icon-btn danger",
                    title: if is_ref { "Unlink (remove this mirror)" } else { "Delete item" },
                    onclick: delete_item,
                    Icon { width: 14, height: 14, icon: LdTrash2 }
                }
            }
            if !disp.note.trim().is_empty() {
                div { class: "wr-item-note", "{disp.note}" }
            }
            if !checklist.is_empty() {
                div { class: "wr-checklist",
                    for (step_idx , step) in checklist.iter().enumerate() {
                        label {
                            key: "{step_idx}",
                            class: "wr-step",
                            if is_ref {
                                input { r#type: "checkbox", checked: step.done, disabled: true }
                            } else {
                                input {
                                    r#type: "checkbox",
                                    checked: step.done,
                                    onchange: {
                                        let item = item.clone();
                                        move |e: Event<FormData>| {
                                            let mut list = item.checklist_items();
                                            if let Some(s) = list.get_mut(step_idx) {
                                                s.done = e.checked();
                                            }
                                            let update = UpdateItemInput {
                                                checklist: Some(list),
                                                ..item_to_update(&item, Some(item.position))
                                            };
                                            spawn(async move {
                                                let _ = api::war_rooms::update_item(item.id, &update).await;
                                                state.invalidate_war_rooms();
                                            });
                                        }
                                    },
                                }
                            }
                            span {
                                class: if step.done { "wr-step-done" } else { "" },
                                "{step.text}"
                            }
                        }
                    }
                }
            }
        }
    }
}
