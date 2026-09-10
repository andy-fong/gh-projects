mod config;
mod db;
mod error;
mod handlers;
mod models;
mod repositories;
mod state;

use std::sync::Arc;
use axum::{
    routing::{delete, get, post, put},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::Config;
use repositories::{
    calendars::SqliteCalendarRepository,
    dashboards::SqliteDashboardRepository,
    notes::SqliteNoteRepository,
    tiles::SqliteTileRepository,
    row_order::SqliteRowOrderRepository,
    repos::SqliteRepoRepository,
    release_watches::SqliteReleaseWatchRepository,
    war_rooms::SqliteWarRoomRepository,
    team_members::SqliteTeamMemberRepository,
};
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env();
    let pool = db::create_pool(&config.database_url).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let state = AppState {
        notes: Arc::new(SqliteNoteRepository::new(pool.clone())),
        dashboards: Arc::new(SqliteDashboardRepository::new(pool.clone())),
        tiles: Arc::new(SqliteTileRepository::new(pool.clone())),
        row_orders: Arc::new(SqliteRowOrderRepository::new(pool.clone())),
        repos: Arc::new(SqliteRepoRepository::new(pool.clone())),
        release_watches: Arc::new(SqliteReleaseWatchRepository::new(pool.clone())),
        war_rooms: Arc::new(SqliteWarRoomRepository::new(pool.clone())),
        calendars: Arc::new(SqliteCalendarRepository::new(pool.clone())),
        team_members: Arc::new(SqliteTeamMemberRepository::new(pool.clone())),
        cache_dir: config.cache_dir.clone(),
        cache_ttl_secs: config.cache_ttl_secs,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // Notes
        .route("/api/notes", get(handlers::notes::list_notes).post(handlers::notes::create_note))
        .route("/api/notes/:id", get(handlers::notes::get_note).put(handlers::notes::update_note).delete(handlers::notes::delete_note))
        // Dashboards
        .route("/api/dashboards", get(handlers::dashboards::list_dashboards).post(handlers::dashboards::create_dashboard))
        .route("/api/dashboards/:id", get(handlers::dashboards::get_dashboard).put(handlers::dashboards::update_dashboard).delete(handlers::dashboards::delete_dashboard))
        // Tiles
        .route("/api/dashboards/:dashboard_id/tiles", get(handlers::tiles::list_tiles).post(handlers::tiles::create_tile))
        .route("/api/dashboards/:dashboard_id/tiles/:tile_id", put(handlers::tiles::update_tile).delete(handlers::tiles::delete_tile))
        // Row order
        .route("/api/tiles/:tile_id/row-order", get(handlers::row_order::get_row_order).put(handlers::row_order::set_row_order))
        // Repo registry (reusable across war rooms)
        .route("/api/repos", get(handlers::repos::list_repos).post(handlers::repos::create_repo))
        .route("/api/repos/:id", put(handlers::repos::update_repo).delete(handlers::repos::delete_repo))
        // Team roster (author groups for GH Query tiles)
        .route("/api/team-members", get(handlers::team_members::list_team_members).post(handlers::team_members::create_team_member))
        .route("/api/team-members/refresh", post(handlers::team_members::refresh_team_members))
        .route("/api/team-members/:id", put(handlers::team_members::update_team_member).delete(handlers::team_members::delete_team_member))
        .route("/api/team-member-sources", get(handlers::team_members::list_team_member_sources).post(handlers::team_members::create_team_member_source))
        .route("/api/team-member-sources/:id", delete(handlers::team_members::delete_team_member_source))
        // Release watches (panel config)
        .route("/api/release-watches", get(handlers::release_watches::list_release_watches).post(handlers::release_watches::create_release_watch))
        .route("/api/release-watches/reorder", put(handlers::release_watches::reorder_release_watches))
        .route("/api/release-watches/:id", delete(handlers::release_watches::delete_release_watch))
        // War rooms
        .route("/api/war-rooms", get(handlers::war_rooms::list_war_rooms).post(handlers::war_rooms::create_war_room))
        .route("/api/war-rooms/:id", get(handlers::war_rooms::get_war_room).put(handlers::war_rooms::update_war_room).delete(handlers::war_rooms::delete_war_room))
        .route("/api/war-rooms/:id/groups", post(handlers::war_rooms::create_group))
        .route("/api/war-room-groups/:id", put(handlers::war_rooms::update_group).delete(handlers::war_rooms::delete_group))
        .route("/api/war-room-groups/:id/items", post(handlers::war_rooms::create_item))
        .route("/api/war-room-items/:id", put(handlers::war_rooms::update_item).delete(handlers::war_rooms::delete_item))
        // Calendar dashboards
        .route("/api/calendars", get(handlers::calendars::list_calendars).post(handlers::calendars::create_calendar))
        .route("/api/calendars/:id", get(handlers::calendars::get_calendar).put(handlers::calendars::update_calendar).delete(handlers::calendars::delete_calendar))
        .route("/api/calendars/:id/components", post(handlers::calendars::create_component))
        .route("/api/calendar-components/:id", put(handlers::calendars::update_component).delete(handlers::calendars::delete_component))
        .route("/api/calendars/:id/events", post(handlers::calendars::create_event))
        .route("/api/calendar-events/:id", put(handlers::calendars::update_event).delete(handlers::calendars::delete_event))
        // GH CLI
        .route("/api/gh/execute", post(handlers::gh::execute_gh))
        // Cache
        .route("/api/cache/invalidate", post(handlers::gh::invalidate_cache))
        .route("/api/cache/status", get(handlers::gh::cache_status))
        // Backup / Restore
        .route("/api/backup", get(handlers::backup::export_backup))
        .route("/api/restore", post(handlers::backup::restore_backup))
        .with_state(state);

    // Serve the compiled Dioxus WASM frontend (built with `dx build --release
    // -p gh-projects-web`). Any request that doesn't match an /api route falls
    // through to the static bundle; unknown paths serve index.html so the
    // client-side router can handle them (SPA fallback). Override the location
    // with STATIC_DIR. If the bundle isn't built yet, the API still runs.
    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| {
        // `dx` writes the bundle under Cargo's target directory, which
        // CARGO_TARGET_DIR relocates — honour it, or a stale `target/dx`
        // bundle gets served silently instead of the one just built.
        let target = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".to_string());
        format!("{target}/dx/gh-projects-web/release/web/public")
    });
    let app = if std::path::Path::new(&static_dir).join("index.html").exists() {
        tracing::info!("Serving frontend bundle from {static_dir}");
        let index = format!("{static_dir}/index.html");
        let serve = ServeDir::new(&static_dir).not_found_service(ServeFile::new(index));
        app.fallback_service(serve)
    } else {
        tracing::warn!(
            "Frontend bundle not found at {static_dir} — serving API only. \
             Build it with `dx build --release -p gh-projects-web` (or set STATIC_DIR)."
        );
        app
    };

    let app = app.layer(cors);

    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("Listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
