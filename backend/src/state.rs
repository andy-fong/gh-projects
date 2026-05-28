use std::sync::Arc;
use std::path::PathBuf;
use crate::repositories::{DashboardRepository, NoteRepository, TileRepository, RowOrderRepository};

#[derive(Clone)]
pub struct AppState {
    pub notes: Arc<dyn NoteRepository>,
    pub dashboards: Arc<dyn DashboardRepository>,
    pub tiles: Arc<dyn TileRepository>,
    pub row_orders: Arc<dyn RowOrderRepository>,
    pub cache_dir: PathBuf,
    pub cache_ttl_secs: u64,
}
