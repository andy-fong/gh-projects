use dioxus::prelude::*;

use crate::api;
use crate::state::use_app_state;
use crate::Route;

/// Mirrors the React root route: redirect to the first dashboard if one exists,
/// otherwise show a hint.
#[component]
pub fn Home() -> Element {
    let state = use_app_state();
    let nav = use_navigator();

    let dashboards = use_resource(move || {
        let _ = state.dashboards_ver.read();
        async move { api::dashboards::list().await }
    });

    use_effect(move || {
        if let Some(Ok(list)) = dashboards.read().as_ref() {
            if let Some(first) = list.first() {
                nav.replace(Route::DashboardPage { id: first.id });
            }
        }
    });

    rsx! {
        div { class: "empty-hint", "Create a dashboard to get started." }
    }
}
