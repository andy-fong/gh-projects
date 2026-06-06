use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{LdClipboardPaste, LdPlus};
use dioxus_free_icons::Icon;
use serde_json::{json, Value};

use crate::api;
use crate::components::dialogs::edit_tile_dialog::EditTileDialog;
use crate::components::dialogs::new_tile_dialog::{NewTileDialog, NewTileInit};
use crate::components::gh_query_tile::GhQueryTile;
use crate::components::note_tile::NoteTile;
use crate::components::tile_wrapper::TileWrapper;
use crate::state::use_app_state;
use crate::types::{CreateTileInput, GhQueryConfig, Tile, UpdateTileInput};

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

    // Mirror the route param into a signal so the resources below refetch when
    // the user navigates between dashboards (route props alone aren't reactive,
    // so without this the view only updates on a full page reload).
    let mut id_sig = use_signal(|| id);
    if *id_sig.peek() != id {
        id_sig.set(id);
    }

    let dashboard = use_resource(move || {
        let _ = state.dashboards_ver.read();
        let id = id_sig();
        async move { api::dashboards::get(id).await }
    });
    let tiles = use_resource(move || {
        let _ = state.tiles_ver.read();
        let id = id_sig();
        async move { api::tiles::list(id).await }
    });

    let mut show_new_tile = use_signal(|| false);
    let mut new_tile_initial = use_signal(|| None::<NewTileInit>);
    let mut copied_tile = use_signal(|| None::<Tile>);
    let mut editing_tile = use_signal(|| None::<Tile>);
    let mut renaming = use_signal(|| false);
    let mut rename_value = use_signal(String::new);

    let dash = match dashboard.read().as_ref() {
        Some(Ok(d)) => Some(d.clone()),
        _ => None,
    };
    let dash_name = dash.as_ref().map(|d| d.name.clone()).unwrap_or_default();
    let tile_list = match tiles.read().as_ref() {
        Some(Ok(v)) => v.clone(),
        _ => Vec::new(),
    };

    let add_tile = move |input: CreateTileInput| {
        spawn(async move {
            if api::tiles::create(id, &input).await.is_ok() {
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
        spawn(async move {
            if api::tiles::update(id, tile.id, &update).await.is_ok() {
                state.invalidate_tiles();
            }
        });
        editing_tile.set(None);
    };

    let delete_tile = move |tile_id: i64| {
        spawn(async move {
            if api::tiles::delete(id, tile_id).await.is_ok() {
                state.invalidate_tiles();
            }
        });
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
                spawn(async move {
                    let _ = api::dashboards::update(
                        id,
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

    let copied = copied_tile.read().clone();

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

            if tile_list.is_empty() {
                div { class: "empty-hint", style: "text-align:center; margin-top:4rem;",
                    "No tiles yet. Add a tile to get started."
                }
            } else {
                div { class: "tile-grid",
                    for tile in tile_list {
                        TileItem {
                            key: "{tile.id}",
                            tile: tile.clone(),
                            on_delete: move |tid| delete_tile(tid),
                            on_edit: move |t: Tile| editing_tile.set(Some(t)),
                            on_copy: move |t: Tile| copied_tile.set(Some(t)),
                        }
                    }
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

/// A single tile: parses its config and renders the right tile body.
#[component]
fn TileItem(
    tile: Tile,
    on_delete: EventHandler<i64>,
    on_edit: EventHandler<Tile>,
    on_copy: EventHandler<Tile>,
) -> Element {
    let tile_id = tile.id;
    let title = tile.title.clone();
    let tile_for_edit = tile.clone();
    let tile_for_copy = tile.clone();

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
            on_delete: move |_| on_delete.call(tile_id),
            on_edit: move |_| on_edit.call(tile_for_edit.clone()),
            on_copy: move |_| on_copy.call(tile_for_copy.clone()),
            {body}
        }
    }
}
