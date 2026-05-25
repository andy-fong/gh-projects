use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use crate::{
    error::AppError,
    models::note::{CreateNoteInput, NoteFilter, UpdateNoteInput},
    state::AppState,
};

pub async fn list_notes(
    State(state): State<AppState>,
    Query(filter): Query<NoteFilter>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let notes = state.notes.list(filter).await?;
    Ok(Json(notes))
}

pub async fn get_note(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    state.notes.find_by_id(id).await?.ok_or(AppError::NotFound).map(Json)
}

pub async fn create_note(
    State(state): State<AppState>,
    Json(input): Json<CreateNoteInput>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let note = state.notes.create(input).await?;
    Ok((StatusCode::CREATED, Json(note)))
}

pub async fn update_note(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateNoteInput>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    state.notes.update(id, input).await?.ok_or(AppError::NotFound).map(Json)
}

pub async fn delete_note(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    if state.notes.delete(id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}
