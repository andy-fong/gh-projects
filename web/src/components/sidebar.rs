use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{
    LdGithub, LdLayoutDashboard, LdPlus, LdRefreshCcw, LdSettings, LdStickyNote,
};
use dioxus_free_icons::Icon;

use crate::api;
use crate::components::dialogs::settings_dialog::SettingsDialog;
use crate::state::use_app_state;
use crate::types::CreateDashboardInput;
use crate::Route;

#[component]
pub fn Sidebar() -> Element {
    let state = use_app_state();
    let route = use_route::<Route>();
    let active_id = match route {
        Route::DashboardPage { id } => Some(id),
        _ => None,
    };

    let dashboards = use_resource(move || {
        let _ = state.dashboards_ver.read(); // subscribe → refetch on invalidate
        async move { api::dashboards::list().await }
    });

    let mut creating = use_signal(|| false);
    let mut new_name = use_signal(String::new);
    let mut invalidating = use_signal(|| false);
    let mut show_settings = use_signal(|| false);

    let submit_create = move |_| {
        let name = new_name.read().trim().to_string();
        if name.is_empty() {
            return;
        }
        spawn(async move {
            if api::dashboards::create(&CreateDashboardInput {
                name,
                description: None,
            })
            .await
            .is_ok()
            {
                new_name.set(String::new());
                creating.set(false);
                state.invalidate_dashboards();
            }
        });
    };

    let invalidate_cache = move |_| {
        invalidating.set(true);
        spawn(async move {
            let _ = api::cache::invalidate().await;
            state.invalidate_gh();
            invalidating.set(false);
        });
    };

    let items = match dashboards.read().as_ref() {
        Some(Ok(v)) => v.clone(),
        _ => Vec::new(),
    };

    rsx! {
        aside { class: "sidebar",
            div { class: "sidebar-brand",
                Icon { width: 20, height: 20, icon: LdGithub }
                span { "GH Projects" }
            }
            nav { class: "sidebar-nav",
                div { class: "sidebar-section", "Dashboards" }
                for d in items {
                    Link {
                        key: "{d.id}",
                        to: Route::DashboardPage { id: d.id },
                        class: if active_id == Some(d.id) { "nav-item active" } else { "nav-item" },
                        Icon { width: 16, height: 16, icon: LdLayoutDashboard }
                        "{d.name}"
                    }
                }
                if creating() {
                    input {
                        class: "input",
                        style: "margin-top:4px;",
                        autofocus: true,
                        placeholder: "Dashboard name…",
                        value: "{new_name}",
                        oninput: move |e| new_name.set(e.value()),
                        onblur: move |_| creating.set(false),
                        onkeydown: move |e| {
                            if e.key() == Key::Enter {
                                submit_create(());
                            } else if e.key() == Key::Escape {
                                creating.set(false);
                            }
                        },
                    }
                } else {
                    button {
                        class: "nav-item subtle",
                        onclick: move |_| creating.set(true),
                        Icon { width: 16, height: 16, icon: LdPlus }
                        "New dashboard"
                    }
                }

                div { class: "sidebar-section", "Tools" }
                Link { to: Route::NotesPage {}, class: "nav-item",
                    Icon { width: 16, height: 16, icon: LdStickyNote }
                    "Notes"
                }
            }
            div { class: "sidebar-footer",
                button {
                    class: "nav-item subtle",
                    disabled: invalidating(),
                    title: "Invalidate all cached gh CLI results",
                    onclick: invalidate_cache,
                    span { class: if invalidating() { "spin" } else { "" },
                        Icon { width: 16, height: 16, icon: LdRefreshCcw }
                    }
                    if invalidating() { "Invalidating…" } else { "Invalidate cache" }
                }
                button {
                    class: "nav-item subtle",
                    onclick: move |_| show_settings.set(true),
                    Icon { width: 16, height: 16, icon: LdSettings }
                    "Settings"
                }
            }
        }
        if show_settings() {
            SettingsDialog { on_close: move |_| show_settings.set(false) }
        }
    }
}
