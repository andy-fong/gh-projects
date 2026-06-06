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
const FAVICON: Asset = asset!("/assets/favicon.svg");

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
