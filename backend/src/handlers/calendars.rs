use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::{
    error::AppError,
    models::calendar::{
        CalendarDashboardDetail, CreateCalendarComponentInput, CreateCalendarDashboardInput,
        CreateCalendarEventInput, UpdateCalendarComponentInput, UpdateCalendarDashboardInput,
        UpdateCalendarEventInput,
    },
    state::AppState,
};

// ── Dashboards ────────────────────────────────────────────────────────────────

pub async fn list_calendars(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(state.calendars.list_dashboards().await?))
}

pub async fn get_calendar(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let dashboard = state.calendars.find_dashboard(id).await?.ok_or(AppError::NotFound)?;
    let components = state.calendars.list_components(id).await?;
    let events = state.calendars.list_events(id).await?;
    Ok(Json(CalendarDashboardDetail { dashboard, components, events }))
}

pub async fn create_calendar(
    State(state): State<AppState>,
    Json(input): Json<CreateCalendarDashboardInput>,
) -> Result<impl IntoResponse, AppError> {
    let d = state.calendars.create_dashboard(input).await?;
    Ok((StatusCode::CREATED, Json(d)))
}

pub async fn update_calendar(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateCalendarDashboardInput>,
) -> Result<impl IntoResponse, AppError> {
    state.calendars.update_dashboard(id, input).await?.ok_or(AppError::NotFound).map(Json)
}

pub async fn delete_calendar(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    state.calendars.delete_dashboard(id).await?.then_some(()).ok_or(AppError::NotFound)?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Components ────────────────────────────────────────────────────────────────

pub async fn create_component(
    State(state): State<AppState>,
    Path(calendar_id): Path<i64>,
    Json(input): Json<CreateCalendarComponentInput>,
) -> Result<impl IntoResponse, AppError> {
    let c = state.calendars.create_component(calendar_id, input).await?;
    Ok((StatusCode::CREATED, Json(c)))
}

pub async fn update_component(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateCalendarComponentInput>,
) -> Result<impl IntoResponse, AppError> {
    state.calendars.update_component(id, input).await?.ok_or(AppError::NotFound).map(Json)
}

pub async fn delete_component(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    state.calendars.delete_component(id).await?.then_some(()).ok_or(AppError::NotFound)?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Events ────────────────────────────────────────────────────────────────────

pub async fn create_event(
    State(state): State<AppState>,
    Path(calendar_id): Path<i64>,
    Json(input): Json<CreateCalendarEventInput>,
) -> Result<impl IntoResponse, AppError> {
    let e = state.calendars.create_event(calendar_id, input).await?;
    Ok((StatusCode::CREATED, Json(e)))
}

pub async fn update_event(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateCalendarEventInput>,
) -> Result<impl IntoResponse, AppError> {
    state.calendars.update_event(id, input).await?.ok_or(AppError::NotFound).map(Json)
}

pub async fn delete_event(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    state.calendars.delete_event(id).await?.then_some(()).ok_or(AppError::NotFound)?;
    Ok(StatusCode::NO_CONTENT)
}
