use dioxus::prelude::*;

mod api;
mod components;
mod datetime;
mod field_extractors;
mod markdown;
mod pages;
mod platform;
mod state;
mod types;
mod war_room;

use components::releases_panel::ReleasesPanel;
use components::sidebar::Sidebar;
use pages::calendar::CalendarPage;
use pages::dashboard::DashboardPage;
use pages::home::Home;
use pages::notes::NotesPage;
use pages::team_stats::TeamStatsPage;
use pages::war_room::WarRoomPage;
use state::AppState;

const MAIN_CSS: Asset = asset!("/assets/main.css");
const FAVICON: Asset = asset!("/assets/favicon.svg");

#[derive(Routable, Clone, PartialEq)]
pub enum Route {
    #[layout(Shell)]
    #[route("/")]
    Home {},
    #[route("/dashboards/:id")]
    DashboardPage { id: i64 },
    #[route("/war-rooms/:id")]
    WarRoomPage { id: i64 },
    #[route("/calendars/:id")]
    CalendarPage { id: i64 },
    #[route("/notes")]
    NotesPage {},
    #[route("/team-stats")]
    TeamStatsPage {},
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Shared state (cache versions + settings) available to the whole tree.
    use_context_provider(AppState::new);

    // Suppress the browser's native context menu inside gh tables so our custom
    // row "Send to Top / Bottom" menu isn't covered by it. A direct, non-passive
    // native listener is used because Dioxus's own `prevent_default()` on
    // `oncontextmenu` doesn't reliably cancel the native menu here.
    use_hook(|| {
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::JsCast;
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            let cb = Closure::<dyn FnMut(web_sys::Event)>::new(|ev: web_sys::Event| {
                if let Some(el) = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
                    if el.closest(".gh-table").ok().flatten().is_some() {
                        ev.prevent_default();
                    }
                }
            });
            let _ = doc
                .add_event_listener_with_callback("contextmenu", cb.as_ref().unchecked_ref());
            cb.forget();
        }
    });

    rsx! {
        document::Stylesheet { href: MAIN_CSS }
        document::Link { rel: "icon", r#type: "image/svg+xml", href: FAVICON }
        Router::<Route> {}
    }
}

/// Top-level layout: sidebar + routed page content + optional releases panel.
#[component]
fn Shell() -> Element {
    let mut show_releases = use_signal(|| false);
    let state = use_context::<AppState>();

    // Load the team roster once for the whole app rather than per tile, and
    // re-fetch whenever `team_ver` is bumped (Team dialog edits / refresh).
    let roster = use_resource(move || {
        let _ = state.team_ver.read();
        async move { api::team_members::list().await }
    });
    use_effect(move || {
        if let Some(Ok(members)) = roster.read().as_ref() {
            let mut state = state;
            state.team_roster.set(
                members
                    .iter()
                    .map(|m| (m.login.to_lowercase(), m.member_group.clone()))
                    .collect(),
            );
        }
    });

    rsx! {
        div { class: "app",
            Sidebar { on_toggle_releases: move |_| show_releases.set(!show_releases()) }
            main { class: "main",
                Outlet::<Route> {}
            }
            if show_releases() {
                ReleasesPanel { on_close: move |_| show_releases.set(false) }
            }
        }
    }
}
