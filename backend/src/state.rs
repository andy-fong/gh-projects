use std::sync::Arc;
use std::path::PathBuf;
use crate::repositories::{DashboardRepository, NoteRepository, TileRepository, RowOrderRepository, RepoRepository, WarRoomRepository};

#[derive(Clone)]
pub struct AppState {
    pub notes: Arc<dyn NoteRepository>,
    pub dashboards: Arc<dyn DashboardRepository>,
    pub tiles: Arc<dyn TileRepository>,
    pub row_orders: Arc<dyn RowOrderRepository>,
    pub repos: Arc<dyn RepoRepository>,
    pub war_rooms: Arc<dyn WarRoomRepository>,
    pub cache_dir: PathBuf,
    pub cache_ttl_secs: u64,
}
