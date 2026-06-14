use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::{
    error::AppError,
    models::release_watch::{CreateReleaseWatchInput, ReorderReleaseWatchesInput},
    state::AppState,
};

pub async fn list_release_watches(
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    Ok(Json(state.release_watches.list().await?))
}

pub async fn create_release_watch(
    State(state): State<AppState>,
    Json(input): Json<CreateReleaseWatchInput>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let watch = state.release_watches.create(input).await?;
    Ok((StatusCode::CREATED, Json(watch)))
}

pub async fn delete_release_watch(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    if state.release_watches.delete(id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

pub async fn reorder_release_watches(
    State(state): State<AppState>,
    Json(input): Json<ReorderReleaseWatchesInput>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    state.release_watches.reorder(&input.ids).await?;
    Ok(Json(serde_json::json!({"ok": true})))
}
