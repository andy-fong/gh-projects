use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use crate::{
    error::AppError,
    models::repo::{CreateRepoInput, UpdateRepoInput},
    state::AppState,
};

pub async fn list_repos(
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    Ok(Json(state.repos.list().await?))
}

pub async fn create_repo(
    State(state): State<AppState>,
    Json(input): Json<CreateRepoInput>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let repo = state.repos.create(input).await?;
    Ok((StatusCode::CREATED, Json(repo)))
}

pub async fn update_repo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateRepoInput>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    state.repos.update(id, input).await?.ok_or(AppError::NotFound).map(Json)
}

pub async fn delete_repo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    if state.repos.delete(id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}
