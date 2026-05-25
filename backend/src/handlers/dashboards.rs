use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use crate::{
    error::AppError,
    models::dashboard::{CreateDashboardInput, UpdateDashboardInput},
    state::AppState,
};

pub async fn list_dashboards(
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    Ok(Json(state.dashboards.list().await?))
}

pub async fn get_dashboard(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    state.dashboards.find_by_id(id).await?.ok_or(AppError::NotFound).map(Json)
}

pub async fn create_dashboard(
    State(state): State<AppState>,
    Json(input): Json<CreateDashboardInput>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let d = state.dashboards.create(input).await?;
    Ok((StatusCode::CREATED, Json(d)))
}

pub async fn update_dashboard(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateDashboardInput>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    state.dashboards.update(id, input).await?.ok_or(AppError::NotFound).map(Json)
}

pub async fn delete_dashboard(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    if state.dashboards.delete(id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}
