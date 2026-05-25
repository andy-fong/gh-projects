use std::sync::Arc;
use crate::repositories::{DashboardRepository, NoteRepository, TileRepository};

#[derive(Clone)]
pub struct AppState {
    pub notes: Arc<dyn NoteRepository>,
    pub dashboards: Arc<dyn DashboardRepository>,
    pub tiles: Arc<dyn TileRepository>,
}
