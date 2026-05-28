mod config;
mod db;
mod error;
mod handlers;
mod models;
mod repositories;
mod state;

use std::sync::Arc;
use axum::{
    routing::{get, post, put},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::Config;
use repositories::{
    dashboards::SqliteDashboardRepository,
    notes::SqliteNoteRepository,
    tiles::SqliteTileRepository,
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
        // GH CLI
        .route("/api/gh/execute", post(handlers::gh::execute_gh))
        // Cache
        .route("/api/cache/invalidate", post(handlers::gh::invalidate_cache))
        .route("/api/cache/status", get(handlers::gh::cache_status))
        // Backup / Restore
        .route("/api/backup", get(handlers::backup::export_backup))
        .route("/api/restore", post(handlers::backup::restore_backup))
        .layer(cors)
        .with_state(state);

    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("Listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
