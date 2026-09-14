use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::{
    error::AppError,
    handlers::gh::run_gh,
    models::team_member::{
        CreateTeamMemberInput, CreateTeamMemberSourceInput, RefreshResult, UpdateTeamMemberInput,
        MEMBER_GROUPS,
    },
    state::AppState,
};

/// The roster has one row per login, so a duplicate is user error rather than
/// a server fault — report it as such instead of a generic database error.
fn map_duplicate(e: AppError, login: &str) -> AppError {
    match &e {
        AppError::Database(sqlx::Error::Database(db)) if db.is_unique_violation() => {
            AppError::BadRequest(format!("{login} is already on the roster"))
        }
        _ => e,
    }
}

/// Reject unknown groups here so a bad value is a 400 rather than a raw
/// SQLite CHECK-constraint failure surfacing as a 500.
fn validate_group(group: &str) -> Result<(), AppError> {
    if MEMBER_GROUPS.contains(&group) {
        Ok(())
    } else {
        Err(AppError::BadRequest(format!(
            "member_group must be one of {}",
            MEMBER_GROUPS.join(", ")
        )))
    }
}

pub async fn list_team_members(
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    Ok(Json(state.team_members.list().await?))
}

pub async fn create_team_member(
    State(state): State<AppState>,
    Json(input): Json<CreateTeamMemberInput>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    validate_group(&input.member_group)?;
    if input.login.trim().is_empty() {
        return Err(AppError::BadRequest("login must not be empty".into()));
    }
    let login = input.login.trim().to_string();
    let member = state
        .team_members
        .create(input)
        .await
        .map_err(|e| map_duplicate(e, &login))?;
    Ok((StatusCode::CREATED, Json(member)))
}

pub async fn update_team_member(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateTeamMemberInput>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    if let Some(g) = &input.member_group {
        validate_group(g)?;
    }
    let login = input.login.clone().unwrap_or_default();
    state
        .team_members
        .update(id, input)
        .await
        .map_err(|e| map_duplicate(e, login.trim()))?
        .ok_or(AppError::NotFound)
        .map(Json)
}

pub async fn delete_team_member(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    if state.team_members.delete(id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

pub async fn list_team_member_sources(
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    Ok(Json(state.team_members.list_sources().await?))
}

pub async fn create_team_member_source(
    State(state): State<AppState>,
    Json(input): Json<CreateTeamMemberSourceInput>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    validate_group(&input.member_group)?;
    if input.org_team.trim().is_empty() {
        return Err(AppError::BadRequest(
            "org_team must be \"owner/team-slug\" or \"owner\"".into(),
        ));
    }
    let source = state.team_members.create_source(input).await?;
    Ok((StatusCode::CREATED, Json(source)))
}

pub async fn delete_team_member_source(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    if state.team_members.delete_source(id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

/// `gh api` command that lists the members of a team (`owner/slug`) or of a
/// whole org (bare `owner`).
fn members_command(org_team: &str) -> String {
    match org_team.split_once('/') {
        Some((org, slug)) => format!("api orgs/{org}/teams/{slug}/members --paginate"),
        None => format!("api orgs/{org_team}/members --paginate"),
    }
}

/// Re-import every configured source. Existing logins keep the group they
/// already have, so a manual `team` assignment is never demoted by a later
/// maintainer import. A failing source is reported but does not abort the rest.
pub async fn refresh_team_members(
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let sources = state.team_members.list_sources().await?;
    let mut result = RefreshResult::default();

    for source in sources {
        let cmd = members_command(&source.org_team);
        let parsed = match run_gh(&cmd).await {
            Ok((parsed, _)) => parsed,
            Err(e) => {
                result.errors.push(format!("{}: {e}", source.org_team));
                continue;
            }
        };
        let Some(arr) = parsed.as_array() else {
            result.errors.push(format!(
                "{}: expected a JSON array from `gh {cmd}`",
                source.org_team
            ));
            continue;
        };
        let logins: Vec<String> = arr
            .iter()
            .filter_map(|m| m.get("login").and_then(|l| l.as_str()))
            .map(String::from)
            .collect();
        if logins.is_empty() {
            result
                .errors
                .push(format!("{}: no logins returned", source.org_team));
            continue;
        }
        let (added, skipped) = state
            .team_members
            .insert_missing(&logins, &source.member_group, &source.org_team)
            .await?;
        result.added += added;
        result.skipped += skipped;
    }

    Ok(Json(result))
}
