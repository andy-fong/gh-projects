use axum::{extract::State, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::{
    error::AppError,
    models::{
        dashboard::CreateDashboardInput,
        note::CreateNoteInput,
        tile::CreateTileInput,
    },
    state::AppState,
};

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

#[derive(Serialize)]
pub struct RestoreDashboardResult {
    pub tile_ids: Vec<i64>,
}

#[derive(Serialize)]
pub struct RestoreResult {
    pub dashboards: Vec<RestoreDashboardResult>,
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

#[derive(Serialize)]
pub struct BackupExport {
    pub version: u32,
    pub exported_at: String,
    pub dashboards: Vec<BackupDashboard>,
    pub notes: Vec<BackupNote>,
}

#[derive(Deserialize)]
pub struct BackupImport {
    pub dashboards: Vec<BackupDashboard>,
    pub notes: Vec<BackupNote>,
}

pub async fn export_backup(
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let dashboards = state.dashboards.list().await?;
    let notes = state.notes.list(Default::default()).await?;

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

    let backup_notes = notes
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

    Ok(Json(BackupExport {
        version: 1,
        exported_at: Utc::now().to_rfc3339(),
        dashboards: backup_dashboards,
        notes: backup_notes,
    }))
}

pub async fn restore_backup(
    State(state): State<AppState>,
    Json(input): Json<BackupImport>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    state.dashboards.delete_all().await?;
    state.notes.delete_all().await?;

    let mut result = RestoreResult { dashboards: Vec::new() };

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

    Ok(Json(result))
}
