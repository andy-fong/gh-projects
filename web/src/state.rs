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
    #[serde(default)]
    stage_emojis: BTreeMap<String, String>,
}

fn load_settings() -> GlobalSettings {
    LocalStorage::get::<GlobalSettings>(SETTINGS_KEY).unwrap_or_default()
}

#[derive(Clone, Copy)]
pub struct AppState {
    pub dashboards_ver: Signal<u32>,
    pub tiles_ver: Signal<u32>,
    pub notes_ver: Signal<u32>,
    pub gh_ver: Signal<u32>,
    pub war_rooms_ver: Signal<u32>,
    pub repos_ver: Signal<u32>,
    pub release_watches_ver: Signal<u32>,
    pub calendars_ver: Signal<u32>,
    pub team_ver: Signal<u32>,
    pub stats_ver: Signal<u32>,
    /// User-defined field extractors (NOT merged with defaults).
    pub user_extractors: Signal<BTreeMap<String, String>>,
    /// User-defined Slack status emojis (NOT merged with defaults).
    pub user_stage_emojis: Signal<BTreeMap<String, String>>,
    /// Team roster: lowercased GitHub login -> "team" | "pe" | "solo" |
    /// "maintainer" | "bot".
    /// Loaded once in `Shell` and refreshed when `team_ver` is bumped; any
    /// login absent from the map is classified "community".
    pub team_roster: Signal<BTreeMap<String, String>>,
}

impl AppState {
    pub fn new() -> Self {
        let settings = load_settings();
        Self {
            dashboards_ver: Signal::new(0),
            tiles_ver: Signal::new(0),
            notes_ver: Signal::new(0),
            gh_ver: Signal::new(0),
            war_rooms_ver: Signal::new(0),
            repos_ver: Signal::new(0),
            release_watches_ver: Signal::new(0),
            calendars_ver: Signal::new(0),
            team_ver: Signal::new(0),
            stats_ver: Signal::new(0),
            user_extractors: Signal::new(settings.field_extractors),
            user_stage_emojis: Signal::new(settings.stage_emojis),
            team_roster: Signal::new(BTreeMap::new()),
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
    pub fn invalidate_war_rooms(mut self) {
        *self.war_rooms_ver.write() += 1;
    }
    pub fn invalidate_repos(mut self) {
        *self.repos_ver.write() += 1;
    }
    pub fn invalidate_release_watches(mut self) {
        *self.release_watches_ver.write() += 1;
    }
    pub fn invalidate_calendars(mut self) {
        *self.calendars_ver.write() += 1;
    }
    pub fn invalidate_team(mut self) {
        *self.team_ver.write() += 1;
    }
    pub fn invalidate_stats(mut self) {
        *self.stats_ver.write() += 1;
    }

    /// Which bucket an item's author falls into, for the `authorGroup` column
    /// in GH Query tiles. Bots win over everything so a bot that happens to be
    /// an org member still reads as a bot; otherwise it is the roster group,
    /// defaulting to "community" for logins we don't know.
    pub fn author_group(&self, login: &str, is_bot: bool) -> &'static str {
        let key = login.to_lowercase();
        let group = self.team_roster.read().get(&key).cloned();
        if is_bot || key.ends_with("[bot]") || group.as_deref() == Some("bot") {
            return "bot";
        }
        match group.as_deref() {
            Some("team") => "team",
            Some("pe") => "pe",
            Some("solo") => "solo",
            Some("maintainer") => "maintainer",
            _ => "community",
        }
    }

    /// Defaults merged with user overrides (defaults first, user wins).
    pub fn global_extractors(&self) -> BTreeMap<String, String> {
        let mut m = default_field_extractors();
        m.extend(self.user_extractors.read().clone());
        m
    }

    /// Stage emojis: defaults merged with user overrides (user wins).
    pub fn stage_emojis(&self) -> BTreeMap<String, String> {
        let mut m = crate::war_room::default_stage_emojis();
        m.extend(self.user_stage_emojis.read().clone());
        m
    }

    /// Persist both settings maps together (they share one storage key).
    fn persist(&self) {
        let settings = GlobalSettings {
            field_extractors: self.user_extractors.read().clone(),
            stage_emojis: self.user_stage_emojis.read().clone(),
        };
        let _ = LocalStorage::set(SETTINGS_KEY, &settings);
    }

    pub fn set_user_extractors(mut self, next: BTreeMap<String, String>) {
        self.user_extractors.set(next);
        self.persist();
    }

    pub fn set_user_stage_emojis(mut self, next: BTreeMap<String, String>) {
        self.user_stage_emojis.set(next);
        self.persist();
    }
}

/// Convenience: read the AppState from context.
pub fn use_app_state() -> AppState {
    use_context::<AppState>()
}
