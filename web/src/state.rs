//! Shared app state provided via Dioxus context.
//!
//! Replaces TanStack Query: instead of a query cache with `invalidateQueries`,
//! each data domain has a `Signal<u32>` "version". A `use_resource` that reads
//! a version subscribes to it; bumping the version re-runs the fetch. This
//! reproduces react-query's invalidate-and-refetch behaviour with plain signals.

use dioxus::prelude::*;
use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::field_extractors::default_field_extractors;

const SETTINGS_KEY: &str = "gh-projects-global-settings";

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct GlobalSettings {
    #[serde(default)]
    field_extractors: BTreeMap<String, String>,
}

fn load_user_extractors() -> BTreeMap<String, String> {
    LocalStorage::get::<GlobalSettings>(SETTINGS_KEY)
        .map(|s| s.field_extractors)
        .unwrap_or_default()
}

#[derive(Clone, Copy)]
pub struct AppState {
    pub dashboards_ver: Signal<u32>,
    pub tiles_ver: Signal<u32>,
    pub notes_ver: Signal<u32>,
    pub gh_ver: Signal<u32>,
    /// User-defined field extractors (NOT merged with defaults).
    pub user_extractors: Signal<BTreeMap<String, String>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            dashboards_ver: Signal::new(0),
            tiles_ver: Signal::new(0),
            notes_ver: Signal::new(0),
            gh_ver: Signal::new(0),
            user_extractors: Signal::new(load_user_extractors()),
        }
    }

    pub fn invalidate_dashboards(mut self) {
        *self.dashboards_ver.write() += 1;
    }
    pub fn invalidate_tiles(mut self) {
        *self.tiles_ver.write() += 1;
    }
    pub fn invalidate_notes(mut self) {
        *self.notes_ver.write() += 1;
    }
    pub fn invalidate_gh(mut self) {
        *self.gh_ver.write() += 1;
    }

    /// Defaults merged with user overrides (defaults first, user wins).
    pub fn global_extractors(&self) -> BTreeMap<String, String> {
        let mut m = default_field_extractors();
        m.extend(self.user_extractors.read().clone());
        m
    }

    pub fn set_user_extractors(mut self, next: BTreeMap<String, String>) {
        let settings = GlobalSettings {
            field_extractors: next.clone(),
        };
        let _ = LocalStorage::set(SETTINGS_KEY, &settings);
        self.user_extractors.set(next);
    }
}

/// Convenience: read the AppState from context.
pub fn use_app_state() -> AppState {
    use_context::<AppState>()
}
