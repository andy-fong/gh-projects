use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{LdClipboardPaste, LdPlus, LdTrash2};
use dioxus_free_icons::Icon;
use hadrone_core::{CompactionType, InteractionPhase, LayoutEvent, LayoutItem};
use hadrone_dioxus::GridLayout;
use serde_json::{json, Value};

use crate::api;
use crate::components::dialogs::edit_tile_dialog::EditTileDialog;
use crate::components::dialogs::new_tile_dialog::{NewTileDialog, NewTileInit};
use crate::components::gh_query_tile::GhQueryTile;
use crate::components::note_tile::NoteTile;
use crate::components::tile_wrapper::TileWrapper;
use crate::state::{use_app_state, AppState};
use crate::types::{CreateTileInput, GhQueryConfig, Tile, TileLayout, UpdateTileInput};
use crate::Route;

/// Context for `render_item` (which must be a plain `fn`, so it can't capture
/// the page's state — it reads everything it needs from here).
#[derive(Clone, Copy)]
struct DashCtx {
    dashboard_id: Signal<i64>,
    tiles: Signal<Vec<Tile>>,
    editing_tile: Signal<Option<Tile>>,
    copied_tile: Signal<Option<Tile>>,
    state: AppState,
}

/// Build the hadrone layout from each tile's persisted `{x,y,w,h}`.
fn build_layout(tiles: &[Tile]) -> Vec<LayoutItem> {
    tiles
        .iter()
        .map(|t| {
            let l: TileLayout = serde_json::from_str(&t.layout).unwrap_or_default();
            LayoutItem {
                id: t.id.to_string(),
                x: l.x,
                y: l.y,
                w: l.w.max(1),
                h: l.h.max(1),
                min_w: Some(2),
                min_h: Some(2),
                ..Default::default()
            }
        })
        .collect()
}

/// Renders the content of one grid cell. `LayoutItem.id` is the tile id.
fn render_tile(item: LayoutItem) -> Element {
    let ctx = use_context::<DashCtx>();
    let tile = ctx
        .tiles
        .read()
        .iter()
        .find(|t| t.id.to_string() == item.id)
        .cloned();
    let Some(tile) = tile else {
        return rsx! {
            div {}
        };
    };

    let tile_id = tile.id;
    let title = tile.title.clone();
    let dash_id = ctx.dashboard_id;
    let state = ctx.state;
    let mut editing = ctx.editing_tile;
    let mut copied = ctx.copied_tile;
    let t_edit = tile.clone();
    let t_copy = tile.clone();

    let body = if tile.tile_type == "gh_query" {
        let config: GhQueryConfig = serde_json::from_str(&tile.config).unwrap_or_default();
        rsx! {
            GhQueryTile { config, tile_id }
        }
    } else {
        let note_id = serde_json::from_str::<Value>(&tile.config)
            .ok()
            .and_then(|v| v.get("note_id").and_then(|n| n.as_i64()))
            .unwrap_or(0);
        rsx! {
            NoteTile { note_id }
        }
    };

    rsx! {
        TileWrapper {
            title,
            on_delete: move |_| {
                let did = dash_id();
                spawn(async move {
                    if api::tiles::delete(did, tile_id).await.is_ok() {
                        state.invalidate_tiles();
                    }
                });
            },
            on_edit: move |_| editing.set(Some(t_edit.clone())),
            on_copy: move |_| copied.set(Some(t_copy.clone())),
            {body}
        }
    }
}

/// Build paste-prefill values from a copied tile. Mirrors `getTilePasteValues`.
fn tile_paste_values(tile: &Tile) -> NewTileInit {
    let cfg: Value = serde_json::from_str(&tile.config).unwrap_or_else(|_| json!({}));
    if tile.tile_type == "gh_query" {
        let commands = cfg
            .get("commands")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
            .filter(|v| !v.is_empty())
            .or_else(|| {
                cfg.get("command")
                    .and_then(|v| v.as_str())
                    .map(|s| vec![s.to_string()])
            })
            .unwrap_or_else(|| vec![String::new()]);
        let variables_json = match cfg.get("variables") {
            Some(v) if v.as_object().map(|o| !o.is_empty()).unwrap_or(false) => {
                serde_json::to_string_pretty(v).unwrap_or_default()
            }
            _ => String::new(),
        };
        NewTileInit {
            tile_type: "gh_query".into(),
            title: format!("{} (copy)", tile.title),
            commands: Some(commands),
            variables_json: Some(variables_json),
            note_id: None,
        }
    } else {
        let note_id = cfg.get("note_id").and_then(|v| v.as_i64()).unwrap_or(0);
        NewTileInit {
            tile_type: "note".into(),
            title: format!("{} (copy)", tile.title),
            commands: None,
            variables_json: None,
            note_id: Some(note_id.to_string()),
        }
    }
}

#[component]
pub fn DashboardPage(id: i64) -> Element {
    let state = use_app_state();

    // Mirror route param into a signal so resources refetch on navigation.
    let mut id_sig = use_signal(|| id);
    if *id_sig.peek() != id {
        id_sig.set(id);
    }

    let dashboard = use_resource(move || {
        let _ = state.dashboards_ver.read();
        let id = id_sig();
        async move { api::dashboards::get(id).await }
    });
    let tiles_res = use_resource(move || {
        let _ = state.tiles_ver.read();
        let id = id_sig();
        async move { api::tiles::list(id).await }
    });

    let nav = use_navigator();
    let mut show_new_tile = use_signal(|| false);
    let mut new_tile_initial = use_signal(|| None::<NewTileInit>);
    let copied_tile = use_signal(|| None::<Tile>);
    let mut editing_tile = use_signal(|| None::<Tile>);
    let mut renaming = use_signal(|| false);
    let mut rename_value = use_signal(String::new);
    let mut confirm_delete = use_signal(|| false);
    let mut tiles_sig = use_signal(Vec::<Tile>::new);
    let mut layout = use_signal(Vec::<LayoutItem>::new);

    // Provide context for `render_tile` (the grid's `render_item` is a plain fn).
    use_context_provider(|| DashCtx {
        dashboard_id: id_sig,
        tiles: tiles_sig,
        editing_tile,
        copied_tile,
        state,
    });

    let dash = match dashboard.read().as_ref() {
        Some(Ok(d)) => Some(d.clone()),
        _ => None,
    };
    let dash_name = dash.as_ref().map(|d| d.name.clone()).unwrap_or_default();
    let tiles_vec = match tiles_res.read().as_ref() {
        Some(Ok(v)) => v.clone(),
        _ => Vec::new(),
    };

    // Keep the tiles context in sync with the fetched tiles (title/config/type).
    if *tiles_sig.peek() != tiles_vec {
        tiles_sig.set(tiles_vec.clone());
    }
    // Rebuild the layout only when the *set* of tile ids changes (add/remove).
    // Position/size edits live in the `layout` signal (mutated by the grid and
    // persisted via on_layout_change) and must not be clobbered on every render.
    {
        let mut now: Vec<i64> = tiles_vec.iter().map(|t| t.id).collect();
        now.sort_unstable();
        let mut have: Vec<i64> = layout
            .peek()
            .iter()
            .filter_map(|i| i.id.parse().ok())
            .collect();
        have.sort_unstable();
        if now != have {
            layout.set(build_layout(&tiles_vec));
        }
    }

    let add_tile = move |input: CreateTileInput| {
        let did = id_sig();
        spawn(async move {
            if api::tiles::create(did, &input).await.is_ok() {
                state.invalidate_tiles();
            }
        });
        show_new_tile.set(false);
        new_tile_initial.set(None);
    };
    let edit_tile = move |update: UpdateTileInput| {
        let Some(tile) = editing_tile.read().clone() else {
            return;
        };
        let did = id_sig();
        spawn(async move {
            if api::tiles::update(did, tile.id, &update).await.is_ok() {
                state.invalidate_tiles();
            }
        });
        editing_tile.set(None);
    };
    let start_rename = {
        let name = dash_name.clone();
        move |_| {
            rename_value.set(name.clone());
            renaming.set(true);
        }
    };
    let commit_rename = {
        let current = dash_name.clone();
        move || {
            let name = rename_value.read().trim().to_string();
            if !name.is_empty() && name != current {
                let did = id_sig();
                spawn(async move {
                    let _ = api::dashboards::update(
                        did,
                        &crate::types::UpdateDashboardInput {
                            name: Some(name),
                            description: None,
                        },
                    )
                    .await;
                    state.invalidate_dashboards();
                });
            }
            renaming.set(false);
        }
    };
    let open_paste = move |_| {
        if let Some(tile) = copied_tile.read().clone() {
            new_tile_initial.set(Some(tile_paste_values(&tile)));
            show_new_tile.set(true);
        }
    };

    // Persist new positions/sizes when a drag or resize ends. (hadrone's
    // `on_layout_change` prop is a no-op in 0.1.1; `on_layout_event` is the
    // real callback — it delivers the final layout on InteractionPhase::Stop.)
    let on_layout_event = move |ev: LayoutEvent| {
        let LayoutEvent::Interaction {
            phase,
            layout: items,
            ..
        } = ev
        else {
            return;
        };
        if matches!(phase, InteractionPhase::Start | InteractionPhase::Update) {
            return;
        }
        let did = id_sig();
        for it in items {
            if let Ok(tid) = it.id.parse::<i64>() {
                let lay = TileLayout {
                    x: it.x,
                    y: it.y,
                    w: it.w,
                    h: it.h,
                };
                spawn(async move {
                    let _ = api::tiles::update(
                        did,
                        tid,
                        &UpdateTileInput {
                            layout: Some(lay),
                            ..Default::default()
                        },
                    )
                    .await;
                });
            }
        }
    };

    let copied = copied_tile.read().clone();
    let has_tiles = !tiles_vec.is_empty();

    rsx! {
        div { class: "page",
            div { class: "page-header",
                if renaming() {
                    input {
                        class: "input",
                        style: "max-width: 320px; font-size:18px; font-weight:600;",
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
                        onclick: start_rename,
                        "{dash_name}"
                    }
                }
                div { class: "toolbar",
                    if let Some(tile) = copied {
                        button {
                            class: "btn btn-sm",
                            title: "Paste \"{tile.title}\"",
                            onclick: open_paste,
                            Icon { width: 16, height: 16, icon: LdClipboardPaste }
                            "Paste tile"
                        }
                    }
                    if confirm_delete() {
                        button {
                            class: "btn btn-danger btn-sm",
                            onclick: move |_| {
                                let did = id_sig();
                                let nav = nav.clone();
                                spawn(async move {
                                    if api::dashboards::delete(did).await.is_ok() {
                                        state.invalidate_dashboards();
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
                            title: "Delete this dashboard",
                            onclick: move |_| confirm_delete.set(true),
                            Icon { width: 16, height: 16, icon: LdTrash2 }
                            "Delete"
                        }
                    }
                    button {
                        class: "btn btn-primary btn-sm",
                        onclick: move |_| {
                            new_tile_initial.set(None);
                            show_new_tile.set(true);
                        },
                        Icon { width: 16, height: 16, icon: LdPlus }
                        "Add tile"
                    }
                }
            }

            if !has_tiles {
                div { class: "empty-hint", style: "text-align:center; margin-top:4rem;",
                    "No tiles yet. Add a tile to get started."
                }
            } else {
                GridLayout {
                    layout,
                    cols: 12,
                    row_height: 60.0,
                    margin: (10, 10),
                    compaction: CompactionType::FreePlacement,
                    render_item: render_tile,
                    on_layout_event,
                }
            }
        }

        if show_new_tile() {
            NewTileDialog {
                initial: new_tile_initial.read().clone(),
                on_confirm: add_tile,
                on_close: move |_| {
                    show_new_tile.set(false);
                    new_tile_initial.set(None);
                },
            }
        }
        if let Some(tile) = editing_tile.read().clone() {
            EditTileDialog {
                tile,
                on_confirm: edit_tile,
                on_close: move |_| editing_tile.set(None),
            }
        }
    }
}
