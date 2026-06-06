use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use crate::{
    error::AppError,
    models::war_room::{
        CreateGroupInput, CreateItemInput, CreateWarRoomInput, GroupWithItems, UpdateGroupInput,
        UpdateItemInput, UpdateWarRoomInput, WarRoomDetail,
    },
    state::AppState,
};

// ---- Rooms ----

pub async fn list_war_rooms(
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    Ok(Json(state.war_rooms.list_rooms().await?))
}

/// Returns the room with its groups, each group carrying its ordered items.
pub async fn get_war_room(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let war_room = state.war_rooms.find_room(id).await?.ok_or(AppError::NotFound)?;
    let groups = state.war_rooms.list_groups(id).await?;
    let mut items = state.war_rooms.list_items(id).await?;

    let groups_with_items = groups
        .into_iter()
        .map(|group| {
            let (mine, rest): (Vec<_>, Vec<_>) =
                items.drain(..).partition(|i| i.group_id == group.id);
            items = rest;
            GroupWithItems { group, items: mine }
        })
        .collect();

    Ok(Json(WarRoomDetail { war_room, groups: groups_with_items }))
}

pub async fn create_war_room(
    State(state): State<AppState>,
    Json(input): Json<CreateWarRoomInput>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let room = state.war_rooms.create_room(input).await?;
    Ok((StatusCode::CREATED, Json(room)))
}

pub async fn update_war_room(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateWarRoomInput>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    state.war_rooms.update_room(id, input).await?.ok_or(AppError::NotFound).map(Json)
}

pub async fn delete_war_room(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    if state.war_rooms.delete_room(id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

// ---- Groups ----

pub async fn create_group(
    State(state): State<AppState>,
    Path(war_room_id): Path<i64>,
    Json(input): Json<CreateGroupInput>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let group = state.war_rooms.create_group(war_room_id, input).await?;
    Ok((StatusCode::CREATED, Json(group)))
}

pub async fn update_group(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateGroupInput>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    state.war_rooms.update_group(id, input).await?.ok_or(AppError::NotFound).map(Json)
}

pub async fn delete_group(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    if state.war_rooms.delete_group(id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

// ---- Items ----

pub async fn create_item(
    State(state): State<AppState>,
    Path(group_id): Path<i64>,
    Json(input): Json<CreateItemInput>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let item = state.war_rooms.create_item(group_id, input).await?;
    Ok((StatusCode::CREATED, Json(item)))
}

pub async fn update_item(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateItemInput>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    state.war_rooms.update_item(id, input).await?.ok_or(AppError::NotFound).map(Json)
}

pub async fn delete_item(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    if state.war_rooms.delete_item(id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}
