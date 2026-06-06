use std::collections::HashMap;

use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{
    LdChevronDown, LdChevronUp, LdExternalLink, LdLink, LdPencil, LdPlus, LdTrash2,
};
use dioxus_free_icons::Icon;

use crate::api;
use crate::components::common::Prose;
use crate::components::dialogs::war_room_group_dialog::WarRoomGroupDialog;
use crate::components::dialogs::war_room_item_dialog::WarRoomItemDialog;
use crate::state::{use_app_state, AppState};
use crate::types::{
    CreateGroupInput, CreateItemInput, GroupWithItems, Repo, UpdateGroupInput, UpdateItemInput,
    UpdateWarRoomInput, WarRoomGroup, WarRoomItem,
};
use crate::war_room::{github_url, stage_badge_class, stage_label};
use crate::Route;

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
    // id -> label, for rendering "depends on" badges.
    dep_labels: Signal<HashMap<i64, String>>,
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
    let mut dep_labels = use_signal(HashMap::<i64, String>::new);
    let mut renaming = use_signal(|| false);
    let mut rename_value = use_signal(String::new);
    let mut confirm_delete = use_signal(|| false);
    let mut editing_links = use_signal(|| false);
    let mut links_value = use_signal(String::new);

    use_context_provider(|| WarCtx {
        state,
        group_dialog,
        item_dialog,
        dep_labels,
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

    // Keep the id -> label map current for "depends on" rendering.
    {
        let map: HashMap<i64, String> = groups
            .iter()
            .flat_map(|g| g.items.iter().map(|i| (i.id, i.label.clone())))
            .collect();
        if *dep_labels.peek() != map {
            dep_labels.set(map);
        }
    }

    // sibling options (id, label) for the item dialog's depends-on select.
    let siblings: Vec<(i64, String)> = groups
        .iter()
        .flat_map(|g| g.items.iter().map(|i| (i.id, i.label.clone())))
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
                        title: "Edit useful links",
                        onclick: {
                            let l = room.links.clone();
                            move |_| {
                                links_value.set(l.clone());
                                editing_links.set(true);
                            }
                        },
                        Icon { width: 16, height: 16, icon: LdLink }
                        "Links"
                    }
                    button {
                        class: "btn btn-primary btn-sm",
                        onclick: move |_| group_dialog.clone().set(Some(None)),
                        Icon { width: 16, height: 16, icon: LdPlus }
                        "Add group"
                    }
                }
            }

            // Useful-links Markdown box — pinned at top when it has content.
            if editing_links() {
                div { class: "wr-links-edit",
                    textarea {
                        class: "textarea mono",
                        rows: 6,
                        autofocus: true,
                        placeholder: "Useful links in Markdown — [Release tracker](https://…), runbooks, dashboards…",
                        value: "{links_value}",
                        oninput: move |e| links_value.set(e.value()),
                    }
                    div { class: "form-actions",
                        button {
                            class: "btn btn-ghost btn-sm",
                            onclick: move |_| editing_links.set(false),
                            "Cancel"
                        }
                        button {
                            class: "btn btn-primary btn-sm",
                            onclick: move |_| {
                                let links = links_value.read().clone();
                                spawn(async move {
                                    let _ = api::war_rooms::update(
                                            room_id,
                                            &UpdateWarRoomInput {
                                                links: Some(links),
                                                ..Default::default()
                                            },
                                        )
                                        .await;
                                    state.invalidate_war_rooms();
                                });
                                editing_links.set(false);
                            },
                            "Save"
                        }
                    }
                }
            } else if !room.links.trim().is_empty() {
                div { class: "wr-links",
                    div { class: "wr-links-head",
                        span { class: "section-label",
                            Icon { width: 14, height: 14, icon: LdLink }
                            "Links"
                        }
                        button {
                            class: "icon-btn",
                            title: "Edit links",
                            onclick: {
                                let l = room.links.clone();
                                move |_| {
                                    links_value.set(l.clone());
                                    editing_links.set(true);
                                }
                            },
                            Icon { width: 14, height: 14, icon: LdPencil }
                        }
                    }
                    Prose { text: room.links.clone(), blank_links: true }
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

    let repo_label = group.repo.clone().unwrap_or_default();
    let item_count = items.len();

    rsx! {
        div { class: "wr-group",
            div { class: "wr-group-head",
                div { class: "wr-group-title",
                    span { class: "wr-group-name", "{group.name}" }
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
    let checklist = item.checklist_items();
    let done_count = checklist.iter().filter(|c| c.done).count();

    let link = match (&repo, item.ref_number) {
        (Some(r), Some(n)) if !r.is_empty() => {
            Some(github_url(r, item.ref_type.as_deref(), n))
        }
        _ => None,
    };

    let dep_label = item
        .depends_on
        .and_then(|d| ctx.dep_labels.read().get(&d).cloned());

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

    let ref_badge = item.ref_number.map(|n| {
        let prefix = if item.ref_type.as_deref() == Some("issue") { "#" } else { "PR #" };
        format!("{prefix}{n}")
    });

    rsx! {
        div { class: "wr-item",
            div { class: "wr-item-main",
                span { class: "badge {stage_badge_class(&item.stage)}", "{stage_label(&item.stage)}" }
                span { class: "wr-item-label", "{item.label}" }
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
                button { class: "icon-btn", title: "Edit item", onclick: edit_item,
                    Icon { width: 14, height: 14, icon: LdPencil }
                }
                button { class: "icon-btn danger", title: "Delete item", onclick: delete_item,
                    Icon { width: 14, height: 14, icon: LdTrash2 }
                }
            }
            if !item.note.trim().is_empty() {
                div { class: "wr-item-note", "{item.note}" }
            }
            if !checklist.is_empty() {
                div { class: "wr-checklist",
                    for (step_idx , step) in checklist.iter().enumerate() {
                        label {
                            key: "{step_idx}",
                            class: "wr-step",
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
