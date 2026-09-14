//! Async HTTP client for the backend REST API. Mirrors `frontend/src/api/client.ts`.
//! Served same-origin, so the base path is just `/api`.

use gloo_net::http::{Request, Response};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::types::*;

/// Parse a response, turning non-2xx into the backend's `{ "error": ... }`
/// message (falling back to status text / raw body).
async fn parse<T: DeserializeOwned>(resp: Response) -> Result<T, String> {
    if !resp.ok() {
        return Err(error_message(resp).await);
    }
    resp.json::<T>().await.map_err(|e| e.to_string())
}

async fn error_message(resp: Response) -> String {
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
        .unwrap_or_else(|| {
            if text.is_empty() {
                format!("HTTP {status}")
            } else {
                text
            }
        })
}

async fn send_get<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    let resp = Request::get(path).send().await.map_err(|e| e.to_string())?;
    parse(resp).await
}

async fn send_json<B: Serialize, T: DeserializeOwned>(
    method: &str,
    path: &str,
    body: &B,
) -> Result<T, String> {
    let builder = match method {
        "POST" => Request::post(path),
        "PUT" => Request::put(path),
        _ => Request::post(path),
    };
    let resp = builder
        .json(body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    parse(resp).await
}

async fn send_delete(path: &str) -> Result<(), String> {
    let resp = Request::delete(path)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.ok() {
        Ok(())
    } else {
        Err(error_message(resp).await)
    }
}

fn enc(s: &str) -> String {
    js_sys::encode_uri_component(s).into()
}

// ---- Notes ----

pub mod notes {
    use super::*;

    #[derive(Default)]
    pub struct Filter {
        pub repo: Option<String>,
        pub ref_type: Option<String>,
        pub ref_number: Option<i64>,
    }

    pub async fn list(filter: Filter) -> Result<Vec<Note>, String> {
        let mut qs = Vec::new();
        if let Some(r) = &filter.repo {
            qs.push(format!("repo={}", enc(r)));
        }
        if let Some(t) = &filter.ref_type {
            qs.push(format!("ref_type={}", enc(t)));
        }
        if let Some(n) = filter.ref_number {
            qs.push(format!("ref_number={n}"));
        }
        let path = if qs.is_empty() {
            "/api/notes".to_string()
        } else {
            format!("/api/notes?{}", qs.join("&"))
        };
        send_get(&path).await
    }

    pub async fn get(id: i64) -> Result<Note, String> {
        send_get(&format!("/api/notes/{id}")).await
    }

    pub async fn create(input: &CreateNoteInput) -> Result<Note, String> {
        send_json("POST", "/api/notes", input).await
    }

    pub async fn update(id: i64, input: &CreateNoteInput) -> Result<Note, String> {
        send_json("PUT", &format!("/api/notes/{id}"), input).await
    }

    pub async fn delete(id: i64) -> Result<(), String> {
        send_delete(&format!("/api/notes/{id}")).await
    }
}

// ---- Dashboards ----

pub mod dashboards {
    use super::*;

    pub async fn list() -> Result<Vec<Dashboard>, String> {
        send_get("/api/dashboards").await
    }

    pub async fn get(id: i64) -> Result<Dashboard, String> {
        send_get(&format!("/api/dashboards/{id}")).await
    }

    pub async fn create(input: &CreateDashboardInput) -> Result<Dashboard, String> {
        send_json("POST", "/api/dashboards", input).await
    }

    pub async fn update(id: i64, input: &UpdateDashboardInput) -> Result<Dashboard, String> {
        send_json("PUT", &format!("/api/dashboards/{id}"), input).await
    }

    #[allow(dead_code)]
    pub async fn delete(id: i64) -> Result<(), String> {
        send_delete(&format!("/api/dashboards/{id}")).await
    }
}

// ---- Tiles ----

pub mod tiles {
    use super::*;

    pub async fn list(dashboard_id: i64) -> Result<Vec<Tile>, String> {
        send_get(&format!("/api/dashboards/{dashboard_id}/tiles")).await
    }

    pub async fn create(dashboard_id: i64, input: &CreateTileInput) -> Result<Tile, String> {
        send_json("POST", &format!("/api/dashboards/{dashboard_id}/tiles"), input).await
    }

    pub async fn update(
        dashboard_id: i64,
        tile_id: i64,
        input: &UpdateTileInput,
    ) -> Result<Tile, String> {
        send_json(
            "PUT",
            &format!("/api/dashboards/{dashboard_id}/tiles/{tile_id}"),
            input,
        )
        .await
    }

    pub async fn delete(dashboard_id: i64, tile_id: i64) -> Result<(), String> {
        send_delete(&format!("/api/dashboards/{dashboard_id}/tiles/{tile_id}")).await
    }
}

// ---- GH CLI ----

pub mod gh {
    use super::*;
    use serde_json::json;

    pub async fn execute(command: &str) -> Result<GhExecuteResponse, String> {
        send_json("POST", "/api/gh/execute", &json!({ "command": command })).await
    }
}

// ---- Cache ----

pub mod cache {
    use super::*;
    use serde_json::Value;

    pub async fn invalidate() -> Result<Value, String> {
        send_json("POST", "/api/cache/invalidate", &serde_json::json!({})).await
    }
}

// ---- Backup / Restore ----

pub mod backup {
    use super::*;
    use serde_json::Value;

    pub async fn export() -> Result<Value, String> {
        send_get("/api/backup").await
    }

    pub async fn restore(data: &Value) -> Result<Value, String> {
        send_json("POST", "/api/restore", data).await
    }
}

// ---- Release watches ----

pub mod release_watches {
    use super::*;

    pub async fn list() -> Result<Vec<ReleaseWatch>, String> {
        send_get("/api/release-watches").await
    }

    pub async fn create(input: &CreateReleaseWatchInput) -> Result<ReleaseWatch, String> {
        send_json("POST", "/api/release-watches", input).await
    }

    pub async fn delete(id: i64) -> Result<(), String> {
        send_delete(&format!("/api/release-watches/{id}")).await
    }

    pub async fn reorder(ids: &[i64]) -> Result<(), String> {
        let _: serde_json::Value =
            send_json("PUT", "/api/release-watches/reorder", &serde_json::json!({ "ids": ids }))
                .await?;
        Ok(())
    }
}

// ---- Repo registry ----

pub mod repos {
    use super::*;

    pub async fn list() -> Result<Vec<Repo>, String> {
        send_get("/api/repos").await
    }

    pub async fn create(input: &CreateRepoInput) -> Result<Repo, String> {
        send_json("POST", "/api/repos", input).await
    }

    pub async fn update(id: i64, input: &UpdateRepoInput) -> Result<Repo, String> {
        send_json("PUT", &format!("/api/repos/{id}"), input).await
    }

    pub async fn delete(id: i64) -> Result<(), String> {
        send_delete(&format!("/api/repos/{id}")).await
    }
}

// ---- Team roster (author groups) ----

pub mod team_members {
    use super::*;

    pub async fn list() -> Result<Vec<TeamMember>, String> {
        send_get("/api/team-members").await
    }

    pub async fn create(input: &CreateTeamMemberInput) -> Result<TeamMember, String> {
        send_json("POST", "/api/team-members", input).await
    }

    pub async fn update(id: i64, input: &UpdateTeamMemberInput) -> Result<TeamMember, String> {
        send_json("PUT", &format!("/api/team-members/{id}"), input).await
    }

    pub async fn delete(id: i64) -> Result<(), String> {
        send_delete(&format!("/api/team-members/{id}")).await
    }

    /// Re-import every configured source. Existing logins keep their group.
    pub async fn refresh() -> Result<RefreshTeamResult, String> {
        send_json("POST", "/api/team-members/refresh", &()).await
    }

    pub async fn list_sources() -> Result<Vec<TeamMemberSource>, String> {
        send_get("/api/team-member-sources").await
    }

    pub async fn create_source(
        input: &CreateTeamMemberSourceInput,
    ) -> Result<TeamMemberSource, String> {
        send_json("POST", "/api/team-member-sources", input).await
    }

    pub async fn delete_source(id: i64) -> Result<(), String> {
        send_delete(&format!("/api/team-member-sources/{id}")).await
    }
}

// ---- War rooms ----

pub mod war_rooms {
    use super::*;

    pub async fn list() -> Result<Vec<WarRoom>, String> {
        send_get("/api/war-rooms").await
    }

    pub async fn get(id: i64) -> Result<WarRoomDetail, String> {
        send_get(&format!("/api/war-rooms/{id}")).await
    }

    pub async fn create(input: &CreateWarRoomInput) -> Result<WarRoom, String> {
        send_json("POST", "/api/war-rooms", input).await
    }

    pub async fn update(id: i64, input: &UpdateWarRoomInput) -> Result<WarRoom, String> {
        send_json("PUT", &format!("/api/war-rooms/{id}"), input).await
    }

    pub async fn delete(id: i64) -> Result<(), String> {
        send_delete(&format!("/api/war-rooms/{id}")).await
    }

    // Groups
    pub async fn create_group(
        war_room_id: i64,
        input: &CreateGroupInput,
    ) -> Result<WarRoomGroup, String> {
        send_json("POST", &format!("/api/war-rooms/{war_room_id}/groups"), input).await
    }

    pub async fn update_group(id: i64, input: &UpdateGroupInput) -> Result<WarRoomGroup, String> {
        send_json("PUT", &format!("/api/war-room-groups/{id}"), input).await
    }

    pub async fn delete_group(id: i64) -> Result<(), String> {
        send_delete(&format!("/api/war-room-groups/{id}")).await
    }

    // Items
    pub async fn create_item(group_id: i64, input: &CreateItemInput) -> Result<WarRoomItem, String> {
        send_json("POST", &format!("/api/war-room-groups/{group_id}/items"), input).await
    }

    pub async fn update_item(id: i64, input: &UpdateItemInput) -> Result<WarRoomItem, String> {
        send_json("PUT", &format!("/api/war-room-items/{id}"), input).await
    }

    pub async fn delete_item(id: i64) -> Result<(), String> {
        send_delete(&format!("/api/war-room-items/{id}")).await
    }
}

// ---- Calendar dashboards ----

pub mod calendars {
    use super::*;

    pub async fn list() -> Result<Vec<CalendarDashboard>, String> {
        send_get("/api/calendars").await
    }

    pub async fn get(id: i64) -> Result<CalendarDashboardDetail, String> {
        send_get(&format!("/api/calendars/{id}")).await
    }

    pub async fn create(input: &CreateCalendarDashboardInput) -> Result<CalendarDashboard, String> {
        send_json("POST", "/api/calendars", input).await
    }

    pub async fn update(id: i64, input: &UpdateCalendarDashboardInput) -> Result<CalendarDashboard, String> {
        send_json("PUT", &format!("/api/calendars/{id}"), input).await
    }

    #[allow(dead_code)]
    pub async fn delete(id: i64) -> Result<(), String> {
        send_delete(&format!("/api/calendars/{id}")).await
    }

    // Components
    pub async fn create_component(calendar_id: i64, input: &CreateCalendarComponentInput) -> Result<CalendarComponent, String> {
        send_json("POST", &format!("/api/calendars/{calendar_id}/components"), input).await
    }

    pub async fn update_component(id: i64, input: &UpdateCalendarComponentInput) -> Result<CalendarComponent, String> {
        send_json("PUT", &format!("/api/calendar-components/{id}"), input).await
    }

    pub async fn delete_component(id: i64) -> Result<(), String> {
        send_delete(&format!("/api/calendar-components/{id}")).await
    }

    // Events
    pub async fn create_event(calendar_id: i64, input: &CreateCalendarEventInput) -> Result<CalendarEvent, String> {
        send_json("POST", &format!("/api/calendars/{calendar_id}/events"), input).await
    }

    pub async fn update_event(id: i64, input: &UpdateCalendarEventInput) -> Result<CalendarEvent, String> {
        send_json("PUT", &format!("/api/calendar-events/{id}"), input).await
    }

    pub async fn delete_event(id: i64) -> Result<(), String> {
        send_delete(&format!("/api/calendar-events/{id}")).await
    }
}

// ---- Row order ----

pub mod row_order {
    use super::*;

    pub async fn get(tile_id: i64) -> Result<RowOrderResponse, String> {
        send_get(&format!("/api/tiles/{tile_id}/row-order")).await
    }

    pub async fn set(tile_id: i64, order: Vec<String>) -> Result<RowOrderResponse, String> {
        send_json(
            "PUT",
            &format!("/api/tiles/{tile_id}/row-order"),
            &serde_json::json!({ "order": order }),
        )
        .await
    }
}

// ---- Team stats ----

pub mod team_stats {
    use super::*;

    fn qs(weeks: Option<i64>, groups: Option<&str>, repos: &[String]) -> String {
        let mut q: Vec<String> = Vec::new();
        if let Some(w) = weeks {
            q.push(format!("weeks={w}"));
        }
        if let Some(g) = groups {
            if g != "all" {
                q.push(format!("groups={g}"));
            }
        }
        if !repos.is_empty() {
            q.push(format!("repos={}", repos.join(",")));
        }
        if q.is_empty() {
            String::new()
        } else {
            format!("?{}", q.join("&"))
        }
    }

    pub async fn summary(
        weeks: i64,
        groups: &str,
        repos: &[String],
    ) -> Result<StatsSummary, String> {
        send_get(&format!(
            "/api/team-stats/summary{}",
            qs(Some(weeks), Some(groups), repos)
        ))
        .await
    }

    pub async fn worklist(
        filter: &str,
        sort: &str,
        repos: &[String],
    ) -> Result<Vec<WorklistRow>, String> {
        let mut url = format!("/api/team-stats/worklist?filter={filter}&sort={sort}&limit=200");
        if !repos.is_empty() {
            url.push_str(&format!("&repos={}", repos.join(",")));
        }
        send_get(&url).await
    }

    pub async fn repos() -> Result<Vec<TrackedRepoStatus>, String> {
        send_get("/api/team-stats/repos").await
    }

    /// Incremental sync of every tracked repo. Slow (tens of seconds on a cold
    /// backfill) — the UI disables its button while this is in flight.
    pub async fn sync() -> Result<SyncResult, String> {
        send_json("POST", "/api/team-stats/sync", &()).await
    }
}
