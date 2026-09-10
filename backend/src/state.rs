use std::sync::Arc;
use std::path::PathBuf;
use crate::repositories::{
    CalendarRepository, DashboardRepository, MaintenanceRepository, NoteRepository,
    ReleaseWatchRepository,
    RepoRepository, RowOrderRepository, TeamMemberRepository, TileRepository,
    WarRoomRepository,
};

#[derive(Clone)]
pub struct AppState {
    pub notes: Arc<dyn NoteRepository>,
    pub dashboards: Arc<dyn DashboardRepository>,
    pub tiles: Arc<dyn TileRepository>,
    pub row_orders: Arc<dyn RowOrderRepository>,
    pub repos: Arc<dyn RepoRepository>,
    pub release_watches: Arc<dyn ReleaseWatchRepository>,
    pub war_rooms: Arc<dyn WarRoomRepository>,
    pub calendars: Arc<dyn CalendarRepository>,
    pub team_members: Arc<dyn TeamMemberRepository>,
    pub maintenance: Arc<dyn MaintenanceRepository>,
    /// Path to the SQLite file, for the pre-restore snapshot. Empty when the
    /// database has no file (e.g. `sqlite::memory:`).
    pub db_path: PathBuf,
    pub cache_dir: PathBuf,
    pub cache_ttl_secs: u64,
}
