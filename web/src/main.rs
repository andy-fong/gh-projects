use dioxus::prelude::*;

mod api;
mod components;
mod field_extractors;
mod markdown;
mod pages;
mod platform;
mod state;
mod types;

use components::sidebar::Sidebar;
use pages::dashboard::DashboardPage;
use pages::home::Home;
use pages::notes::NotesPage;
use state::AppState;

const MAIN_CSS: Asset = asset!("/assets/main.css");

#[derive(Routable, Clone, PartialEq)]
pub enum Route {
    #[layout(Shell)]
    #[route("/")]
    Home {},
    #[route("/dashboards/:id")]
    DashboardPage { id: i64 },
    #[route("/notes")]
    NotesPage {},
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Shared state (cache versions + settings) available to the whole tree.
    use_context_provider(AppState::new);

    rsx! {
        document::Stylesheet { href: MAIN_CSS }
        Router::<Route> {}
    }
}

/// Top-level layout: sidebar + routed page content.
#[component]
fn Shell() -> Element {
    rsx! {
        div { class: "app",
            Sidebar {}
            main { class: "main",
                Outlet::<Route> {}
            }
        }
    }
}
