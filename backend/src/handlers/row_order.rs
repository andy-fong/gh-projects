use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use crate::{error::AppError, state::AppState};

#[derive(Serialize)]
pub struct RowOrderResponse {
    pub order: Vec<String>,
}

#[derive(Deserialize)]
pub struct SetRowOrderInput {
    pub order: Vec<String>,
}

pub async fn get_row_order(
    State(state): State<AppState>,
    Path(tile_id): Path<i64>,
) -> Result<Json<RowOrderResponse>, AppError> {
    let order = state.row_orders.get(tile_id).await?;
    Ok(Json(RowOrderResponse { order }))
}

pub async fn set_row_order(
    State(state): State<AppState>,
    Path(tile_id): Path<i64>,
    Json(input): Json<SetRowOrderInput>,
) -> Result<Json<RowOrderResponse>, AppError> {
    let order = state.row_orders.set(tile_id, input.order).await?;
    Ok(Json(RowOrderResponse { order }))
}
