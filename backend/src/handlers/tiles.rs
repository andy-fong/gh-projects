use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use crate::{
    error::AppError,
    models::tile::{CreateTileInput, UpdateTileInput},
    state::AppState,
};

pub async fn list_tiles(
    State(state): State<AppState>,
    Path(dashboard_id): Path<i64>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    Ok(Json(state.tiles.list_by_dashboard(dashboard_id).await?))
}

pub async fn create_tile(
    State(state): State<AppState>,
    Path(dashboard_id): Path<i64>,
    Json(input): Json<CreateTileInput>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let tile = state.tiles.create(dashboard_id, input).await?;
    Ok((StatusCode::CREATED, Json(tile)))
}

pub async fn update_tile(
    State(state): State<AppState>,
    Path((_dashboard_id, tile_id)): Path<(i64, i64)>,
    Json(input): Json<UpdateTileInput>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    state.tiles.update(tile_id, input).await?.ok_or(AppError::NotFound).map(Json)
}

pub async fn delete_tile(
    State(state): State<AppState>,
    Path((_dashboard_id, tile_id)): Path<(i64, i64)>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    if state.tiles.delete(tile_id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}
