use std::collections::{HashMap, HashSet};
use std::path::Path;
use axum::{extract::State, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::{
    error::AppError,
    models::{
        calendar::{CreateCalendarComponentInput, CreateCalendarDashboardInput, CreateCalendarEventInput},
        dashboard::CreateDashboardInput,
        note::CreateNoteInput,
        release_watch::CreateReleaseWatchInput,
        repo::CreateRepoInput,
        team_member::{CreateTeamMemberInput, CreateTeamMemberSourceInput, MEMBER_GROUPS},
        tile::CreateTileInput,
        war_room::{ChecklistItem, CreateGroupInput, CreateItemInput, CreateWarRoomInput, UpdateWarRoomInput},
    },
    state::AppState,
};

// ── Grafana-style dashboards ───────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct BackupTile {
    pub id: i64,
    pub title: String,
    pub tile_type: String,
    pub config: Value,
    pub layout: Value,
    #[serde(default)]
    pub row_order: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct BackupDashboard {
    pub name: String,
    pub description: String,
    pub tiles: Vec<BackupTile>,
}

#[derive(Serialize, Deserialize)]
pub struct BackupNote {
    pub title: String,
    pub body: String,
    pub repo: Option<String>,
    pub ref_type: Option<String>,
    pub ref_number: Option<i64>,
    pub tags: Vec<String>,
}

// ── War rooms ─────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct BackupWarRoomItem {
    /// Original DB id — used to remap `depends_on` references on restore.
    pub orig_id: i64,
    pub label: String,
    pub ref_type: Option<String>,
    pub ref_number: Option<i64>,
    pub note: String,
    pub stage: String,
    /// Parsed checklist array (not the raw JSON string stored in the DB).
    pub checklist: Value,
    /// References `orig_id` of another item in the same group.
    pub depends_on: Option<i64>,
    pub source_item_id: Option<i64>,
}

#[derive(Serialize, Deserialize)]
pub struct BackupWarRoomGroup {
    pub name: String,
    pub repo_id: Option<i64>,
    pub repo: Option<String>,
    pub items: Vec<BackupWarRoomItem>,
}

#[derive(Serialize, Deserialize)]
pub struct BackupWarRoom {
    pub name: String,
    pub description: String,
    pub status: String,
    pub notes: String,
    pub groups: Vec<BackupWarRoomGroup>,
}

// ── Calendar dashboards ───────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct BackupCalendarComponent {
    /// Original DB id — used to remap `component_id` on events during restore.
    pub orig_id: i64,
    pub name: String,
    pub short_name: Option<String>,
    pub color: String,
}

#[derive(Serialize, Deserialize)]
pub struct BackupCalendarEvent {
    /// References `orig_id` of a component; None means no component association.
    pub component_id: Option<i64>,
    pub version: String,
    pub status: String,
    pub release_date: String,
    pub actual_release_date: Option<String>,
    pub note: String,
}

#[derive(Serialize, Deserialize)]
pub struct BackupCalendar {
    pub name: String,
    pub description: String,
    pub components: Vec<BackupCalendarComponent>,
    pub events: Vec<BackupCalendarEvent>,
}

// ── Release watches ───────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct BackupReleaseWatch {
    pub repo_id: i64,
    pub limit_count: i64,
}

// ── Repo registry ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct BackupRepo {
    /// Original DB id — war-room groups and release watches reference it.
    pub orig_id: i64,
    pub name: String,
    pub owner_repo: String,
}

// ── Team roster ───────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct BackupTeamMember {
    pub login: String,
    pub member_group: String,
    pub source: String,
}

#[derive(Serialize, Deserialize)]
pub struct BackupTeamMemberSource {
    pub member_group: String,
    pub org_team: String,
}

// ── Top-level export / import ─────────────────────────────────────────────────

#[derive(Serialize)]
pub struct BackupExport {
    pub version: u32,
    pub exported_at: String,
    pub dashboards: Vec<BackupDashboard>,
    pub notes: Vec<BackupNote>,
    pub war_rooms: Vec<BackupWarRoom>,
    pub calendars: Vec<BackupCalendar>,
    pub release_watches: Vec<BackupReleaseWatch>,
    pub repos: Vec<BackupRepo>,
    pub team_members: Vec<BackupTeamMember>,
    pub team_member_sources: Vec<BackupTeamMemberSource>,
}

#[derive(Deserialize)]
pub struct BackupImport {
    pub dashboards: Vec<BackupDashboard>,
    pub notes: Vec<BackupNote>,
    #[serde(default)]
    pub war_rooms: Vec<BackupWarRoom>,
    #[serde(default)]
    pub calendars: Vec<BackupCalendar>,
    #[serde(default)]
    pub release_watches: Vec<BackupReleaseWatch>,
    /// Absent in v2/v3 files; then `repo_id`s are treated as raw local ids.
    #[serde(default)]
    pub repos: Vec<BackupRepo>,
    #[serde(default)]
    pub team_members: Vec<BackupTeamMember>,
    #[serde(default)]
    pub team_member_sources: Vec<BackupTeamMemberSource>,
}

// ── Restore result ────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct RestoreDashboardResult {
    pub tile_ids: Vec<i64>,
}

#[derive(Serialize)]
pub struct RestoreResult {
    pub dashboards: Vec<RestoreDashboardResult>,
    /// Where the pre-restore database copy was written, if one was taken.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
    /// Cross-item links (`depends_on` / `source_item_id`) re-pointed at the
    /// newly created rows.
    pub links_restored: usize,
    /// Links whose target wasn't present in the backup, so they were cleared.
    pub links_dropped: usize,
    /// Release watches skipped because their repo couldn't be resolved.
    pub release_watches_skipped: usize,
}

// ── Restore helpers ───────────────────────────────────────────────────────────

/// Reject a payload that would certainly fail part-way through, while the
/// database is still intact. Only checks constraints the DB itself enforces —
/// unresolvable references are handled by clearing them, not by failing.
fn validate_import(input: &BackupImport) -> Result<(), AppError> {
    for d in &input.dashboards {
        for t in &d.tiles {
            if t.tile_type != "gh_query" && t.tile_type != "note" {
                return Err(AppError::BadRequest(format!(
                    "dashboard \"{}\": unknown tile_type \"{}\"",
                    d.name, t.tile_type
                )));
            }
        }
    }
    for wr in &input.war_rooms {
        for g in &wr.groups {
            for i in &g.items {
                if let Some(rt) = &i.ref_type {
                    if rt != "pr" && rt != "issue" {
                        return Err(AppError::BadRequest(format!(
                            "war room \"{}\", item \"{}\": unknown ref_type \"{rt}\"",
                            wr.name, i.label
                        )));
                    }
                }
            }
        }
    }
    for n in &input.notes {
        if let Some(rt) = &n.ref_type {
            if rt != "pr" && rt != "issue" {
                return Err(AppError::BadRequest(format!(
                    "note \"{}\": unknown ref_type \"{rt}\"",
                    n.title
                )));
            }
        }
    }
    for m in &input.team_members {
        if !MEMBER_GROUPS.contains(&m.member_group.as_str()) {
            return Err(AppError::BadRequest(format!(
                "team member \"{}\": unknown member_group \"{}\"",
                m.login, m.member_group
            )));
        }
    }
    for src in &input.team_member_sources {
        if !MEMBER_GROUPS.contains(&src.member_group.as_str()) {
            return Err(AppError::BadRequest(format!(
                "import source \"{}\": unknown member_group \"{}\"",
                src.org_team, src.member_group
            )));
        }
    }
    Ok(())
}

/// Where to put the pre-restore copy: alongside the database, stamped so it
/// never overwrites an earlier one. `None` for a database with no file path.
fn snapshot_path(db_path: &Path) -> Option<String> {
    let name = db_path.file_name()?.to_str()?;
    let stamp = Utc::now().format("%Y%m%dT%H%M%S");
    Some(
        db_path
            .with_file_name(format!("{name}.pre-restore-{stamp}"))
            .to_str()?
            .to_string(),
    )
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn export_backup(
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    // Grafana dashboards
    let dashboards = state.dashboards.list().await?;
    let mut backup_dashboards = Vec::new();
    for d in dashboards {
        let tiles = state.tiles.list_by_dashboard(d.id).await?;
        let mut backup_tiles = Vec::new();
        for t in tiles {
            let row_order = state.row_orders.get(t.id).await.unwrap_or_default();
            backup_tiles.push(BackupTile {
                id: t.id,
                title: t.title,
                tile_type: t.tile_type,
                config: serde_json::from_str(&t.config).unwrap_or(Value::Null),
                layout: serde_json::from_str(&t.layout).unwrap_or(Value::Null),
                row_order,
            });
        }
        backup_dashboards.push(BackupDashboard {
            name: d.name,
            description: d.description,
            tiles: backup_tiles,
        });
    }

    // Notes
    let backup_notes = state
        .notes
        .list(Default::default())
        .await?
        .into_iter()
        .map(|n| BackupNote {
            title: n.title,
            body: n.body,
            repo: n.repo,
            ref_type: n.ref_type,
            ref_number: n.ref_number,
            tags: serde_json::from_str(&n.tags).unwrap_or_default(),
        })
        .collect();

    // War rooms
    let rooms = state.war_rooms.list_rooms().await?;
    let mut backup_war_rooms = Vec::new();
    for room in rooms {
        let groups = state.war_rooms.list_groups(room.id).await?;
        let items = state.war_rooms.list_items(room.id).await?;
        let mut backup_groups = Vec::new();
        for g in &groups {
            let group_items = items
                .iter()
                .filter(|i| i.group_id == g.id)
                .map(|i| BackupWarRoomItem {
                    orig_id: i.id,
                    label: i.label.clone(),
                    ref_type: i.ref_type.clone(),
                    ref_number: i.ref_number,
                    note: i.note.clone(),
                    stage: i.stage.clone(),
                    checklist: serde_json::from_str(&i.checklist)
                        .unwrap_or(Value::Array(vec![])),
                    depends_on: i.depends_on,
                    source_item_id: i.source_item_id,
                })
                .collect();
            backup_groups.push(BackupWarRoomGroup {
                name: g.name.clone(),
                repo_id: g.repo_id,
                repo: g.repo.clone(),
                items: group_items,
            });
        }
        backup_war_rooms.push(BackupWarRoom {
            name: room.name,
            description: room.description,
            status: room.status,
            notes: room.notes,
            groups: backup_groups,
        });
    }

    // Calendar dashboards
    let cal_dashboards = state.calendars.list_dashboards().await?;
    let mut backup_calendars = Vec::new();
    for cal in cal_dashboards {
        let components = state.calendars.list_components(cal.id).await?;
        let events = state.calendars.list_events(cal.id).await?;
        let backup_components = components
            .iter()
            .map(|c| BackupCalendarComponent {
                orig_id: c.id,
                name: c.name.clone(),
                short_name: c.short_name.clone(),
                color: c.color.clone(),
            })
            .collect();
        let backup_events = events
            .iter()
            .map(|e| BackupCalendarEvent {
                component_id: e.component_id,
                version: e.version.clone(),
                status: e.status.clone(),
                release_date: e.release_date.clone(),
                actual_release_date: e.actual_release_date.clone(),
                note: e.note.clone(),
            })
            .collect();
        backup_calendars.push(BackupCalendar {
            name: cal.name,
            description: cal.description,
            components: backup_components,
            events: backup_events,
        });
    }

    let backup_release_watches = state
        .release_watches
        .list()
        .await?
        .into_iter()
        .map(|w| BackupReleaseWatch {
            repo_id: w.repo_id,
            limit_count: w.limit_count,
        })
        .collect();

    let backup_team_members = state
        .team_members
        .list()
        .await?
        .into_iter()
        .map(|m| BackupTeamMember {
            login: m.login,
            member_group: m.member_group,
            source: m.source,
        })
        .collect();

    let backup_team_member_sources = state
        .team_members
        .list_sources()
        .await?
        .into_iter()
        .map(|s| BackupTeamMemberSource {
            member_group: s.member_group,
            org_team: s.org_team,
        })
        .collect();

    // Repo registry — war-room groups and release watches point at these by
    // id, so a backup without them can't be restored onto a different DB.
    let backup_repos = state
        .repos
        .list()
        .await?
        .into_iter()
        .map(|r| BackupRepo {
            orig_id: r.id,
            name: r.name,
            owner_repo: r.owner_repo,
        })
        .collect();

    Ok(Json(BackupExport {
        version: 4,
        exported_at: Utc::now().to_rfc3339(),
        dashboards: backup_dashboards,
        notes: backup_notes,
        war_rooms: backup_war_rooms,
        calendars: backup_calendars,
        release_watches: backup_release_watches,
        repos: backup_repos,
        team_members: backup_team_members,
        team_member_sources: backup_team_member_sources,
    }))
}

pub async fn restore_backup(
    State(state): State<AppState>,
    Json(input): Json<BackupImport>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    // ---- Validate before touching anything -------------------------------
    // Restore is destructive and not transactional, so anything that would
    // certainly fail mid-way must be rejected while the DB is still intact.
    validate_import(&input)?;

    // ---- Snapshot the current database ------------------------------------
    let snapshot = match snapshot_path(&state.db_path) {
        Some(path) => match state.maintenance.snapshot_to(&path).await {
            Ok(()) => Some(path),
            Err(e) => {
                // A snapshot is a safety net, not a requirement — an in-memory
                // or unwritable DB shouldn't block a restore.
                tracing::warn!("Pre-restore snapshot failed: {e}");
                None
            }
        },
        None => None,
    };

    // ---- Resolve the repo registry ----------------------------------------
    // Done before the wipe because `repos` is a global registry that restore
    // deliberately does NOT clear: wiping it would cascade-delete release
    // watches and null out war-room group links. Existing rows are matched by
    // `owner_repo`; only genuinely new ones are created.
    let mut repo_id_map: HashMap<i64, i64> = HashMap::new();
    if !input.repos.is_empty() {
        let existing = state.repos.list().await?;
        for br in &input.repos {
            let want = br.owner_repo.trim().to_lowercase();
            let hit = existing
                .iter()
                .find(|r| r.owner_repo.trim().to_lowercase() == want)
                .map(|r| r.id);
            let id = match hit {
                Some(id) => id,
                None => {
                    state
                        .repos
                        .create(CreateRepoInput {
                            name: br.name.clone(),
                            owner_repo: br.owner_repo.clone(),
                        })
                        .await?
                        .id
                }
            };
            repo_id_map.insert(br.orig_id, id);
        }
    }
    // Ids that survive as-is when the backup predates `repos` (v2/v3 files).
    let live_repo_ids: HashSet<i64> = state.repos.list().await?.iter().map(|r| r.id).collect();
    let resolve_repo = |orig: i64| -> Option<i64> {
        repo_id_map
            .get(&orig)
            .copied()
            .or_else(|| live_repo_ids.contains(&orig).then_some(orig))
    };

    // Delete all existing data (FK CASCADE handles children)
    state.dashboards.delete_all().await?;
    state.notes.delete_all().await?;
    state.war_rooms.delete_all().await?;
    state.calendars.delete_all().await?;
    state.release_watches.delete_all().await?;
    state.team_members.delete_all().await?;
    state.team_members.delete_all_sources().await?;

    let mut result = RestoreResult {
        dashboards: Vec::new(),
        snapshot,
        links_restored: 0,
        links_dropped: 0,
        release_watches_skipped: 0,
    };

    // Restore Grafana dashboards
    for d in input.dashboards {
        let dashboard = state
            .dashboards
            .create(CreateDashboardInput {
                name: d.name,
                description: Some(d.description),
            })
            .await?;
        let mut tile_ids = Vec::new();
        for t in d.tiles {
            let layout = serde_json::from_value(t.layout).ok();
            let tile = state
                .tiles
                .create(
                    dashboard.id,
                    CreateTileInput {
                        title: t.title,
                        tile_type: t.tile_type,
                        config: t.config,
                        layout,
                    },
                )
                .await?;
            if !t.row_order.is_empty() {
                state.row_orders.set(tile.id, t.row_order).await?;
            }
            tile_ids.push(tile.id);
        }
        result.dashboards.push(RestoreDashboardResult { tile_ids });
    }

    // Restore notes
    for n in input.notes {
        state
            .notes
            .create(CreateNoteInput {
                title: n.title,
                body: Some(n.body),
                repo: n.repo,
                ref_type: n.ref_type,
                ref_number: n.ref_number,
                tags: Some(n.tags),
            })
            .await?;
    }

    // Restore war rooms.
    //
    // Two passes: `depends_on` and `source_item_id` routinely point at items
    // in a *different* group or war room, so nothing can be linked until every
    // item exists. Pass 1 creates all items unlinked and records
    // orig_id -> new_id globally; pass 2 re-points the links.
    let mut item_id_map: HashMap<i64, i64> = HashMap::new();
    let mut pending_links: Vec<(i64, Option<i64>, Option<i64>)> = Vec::new();
    for wr in input.war_rooms {
        let room = state
            .war_rooms
            .create_room(CreateWarRoomInput {
                name: wr.name.clone(),
                description: if wr.description.is_empty() {
                    None
                } else {
                    Some(wr.description.clone())
                },
            })
            .await?;

        // Set status and notes via update (create only accepts name/description)
        state
            .war_rooms
            .update_room(
                room.id,
                UpdateWarRoomInput {
                    name: None,
                    description: None,
                    status: Some(wr.status.clone()),
                    config: None,
                    notes: if wr.notes.is_empty() {
                        None
                    } else {
                        Some(wr.notes.clone())
                    },
                    position: None,
                },
            )
            .await?;

        for g in wr.groups {
            let group = state
                .war_rooms
                .create_group(
                    room.id,
                    CreateGroupInput {
                        name: g.name.clone(),
                        repo_id: g.repo_id.and_then(resolve_repo),
                        repo: g.repo.clone(),
                    },
                )
                .await?;

            for item in &g.items {
                let checklist: Vec<ChecklistItem> =
                    serde_json::from_value(item.checklist.clone()).unwrap_or_default();
                let new_item = state
                    .war_rooms
                    .create_item(
                        group.id,
                        CreateItemInput {
                            label: item.label.clone(),
                            ref_type: item.ref_type.clone(),
                            ref_number: item.ref_number,
                            note: if item.note.is_empty() {
                                None
                            } else {
                                Some(item.note.clone())
                            },
                            stage: if item.stage.is_empty() {
                                None
                            } else {
                                Some(item.stage.clone())
                            },
                            checklist: Some(checklist),
                            // Linked in pass 2, once every target exists.
                            depends_on: None,
                            source_item_id: None,
                        },
                    )
                    .await?;
                item_id_map.insert(item.orig_id, new_item.id);
                if item.depends_on.is_some() || item.source_item_id.is_some() {
                    pending_links.push((new_item.id, item.depends_on, item.source_item_id));
                }
            }
        }
    }

    // Pass 2: re-point cross-item links now that every item has a new id. A
    // target missing from the backup is cleared rather than left dangling.
    for (new_id, orig_depends, orig_source) in pending_links {
        let depends_on = orig_depends.and_then(|d| item_id_map.get(&d).copied());
        let source_item_id = orig_source.and_then(|s| item_id_map.get(&s).copied());
        result.links_restored += depends_on.is_some() as usize + source_item_id.is_some() as usize;
        result.links_dropped += (orig_depends.is_some() && depends_on.is_none()) as usize
            + (orig_source.is_some() && source_item_id.is_none()) as usize;
        state
            .war_rooms
            .relink_item(new_id, depends_on, source_item_id)
            .await?;
    }

    // Restore calendar dashboards
    for cal in input.calendars {
        let dashboard = state
            .calendars
            .create_dashboard(CreateCalendarDashboardInput {
                name: cal.name.clone(),
                description: if cal.description.is_empty() {
                    None
                } else {
                    Some(cal.description.clone())
                },
            })
            .await?;

        // Components first; build old-id → new-id map for event remapping
        let mut comp_id_map: HashMap<i64, i64> = HashMap::new();
        for comp in &cal.components {
            let new_comp = state
                .calendars
                .create_component(
                    dashboard.id,
                    CreateCalendarComponentInput {
                        name: comp.name.clone(),
                        short_name: comp.short_name.clone(),
                        color: Some(comp.color.clone()),
                    },
                )
                .await?;
            comp_id_map.insert(comp.orig_id, new_comp.id);
        }

        for event in &cal.events {
            let remapped_comp = event.component_id.and_then(|id| comp_id_map.get(&id).copied());
            state
                .calendars
                .create_event(
                    dashboard.id,
                    CreateCalendarEventInput {
                        component_id: remapped_comp,
                        version: event.version.clone(),
                        status: Some(event.status.clone()),
                        release_date: event.release_date.clone(),
                        actual_release_date: event.actual_release_date.clone(),
                        note: if event.note.is_empty() {
                            None
                        } else {
                            Some(event.note.clone())
                        },
                    },
                )
                .await?;
        }
    }

    // Restore release watches (position is implicit from order). `repo_id` is
    // NOT NULL, so a watch whose repo can't be resolved is skipped and counted
    // rather than failing the whole restore.
    for w in input.release_watches {
        match resolve_repo(w.repo_id) {
            Some(repo_id) => {
                state
                    .release_watches
                    .create(CreateReleaseWatchInput {
                        repo_id,
                        limit_count: w.limit_count,
                    })
                    .await?;
            }
            None => result.release_watches_skipped += 1,
        }
    }

    // Restore the team roster and its import sources
    for m in input.team_members {
        state
            .team_members
            .create(CreateTeamMemberInput {
                login: m.login,
                member_group: m.member_group,
                source: Some(m.source),
            })
            .await?;
    }
    for s in input.team_member_sources {
        state
            .team_members
            .create_source(CreateTeamMemberSourceInput {
                member_group: s.member_group,
                org_team: s.org_team,
            })
            .await?;
    }

    Ok(Json(result))
}
