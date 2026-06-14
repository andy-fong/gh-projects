use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{
    LdCalendar, LdFolderGit2, LdGithub, LdLayoutDashboard, LdPlus, LdRefreshCcw, LdSettings,
    LdSiren, LdStickyNote,
};
use dioxus_free_icons::Icon;

use crate::api;
use crate::components::dialogs::repos_dialog::ReposDialog;
use crate::components::dialogs::settings_dialog::SettingsDialog;
use crate::state::use_app_state;
use crate::types::{CreateCalendarDashboardInput, CreateDashboardInput, CreateWarRoomInput};
use crate::Route;

#[component]
pub fn Sidebar() -> Element {
    let state = use_app_state();
    let route = use_route::<Route>();
    let active_id = match route {
        Route::DashboardPage { id } => Some(id),
        _ => None,
    };
    let active_wr = match route {
        Route::WarRoomPage { id } => Some(id),
        _ => None,
    };
    let active_cal = match route {
        Route::CalendarPage { id } => Some(id),
        _ => None,
    };

    let dashboards = use_resource(move || {
        let _ = state.dashboards_ver.read(); // subscribe → refetch on invalidate
        async move { api::dashboards::list().await }
    });
    let war_rooms = use_resource(move || {
        let _ = state.war_rooms_ver.read();
        async move { api::war_rooms::list().await }
    });
    let calendars = use_resource(move || {
        let _ = state.calendars_ver.read();
        async move { api::calendars::list().await }
    });

    let mut creating = use_signal(|| false);
    let mut new_name = use_signal(String::new);
    let mut creating_wr = use_signal(|| false);
    let mut new_wr_name = use_signal(String::new);
    let mut creating_cal = use_signal(|| false);
    let mut new_cal_name = use_signal(String::new);
    let mut invalidating = use_signal(|| false);
    let mut show_settings = use_signal(|| false);
    let mut show_repos = use_signal(|| false);

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

    let submit_create_wr = move |_| {
        let name = new_wr_name.read().trim().to_string();
        if name.is_empty() {
            return;
        }
        spawn(async move {
            if api::war_rooms::create(&CreateWarRoomInput {
                name,
                description: None,
            })
            .await
            .is_ok()
            {
                new_wr_name.set(String::new());
                creating_wr.set(false);
                state.invalidate_war_rooms();
            }
        });
    };

    let submit_create_cal = move |_| {
        let name = new_cal_name.read().trim().to_string();
        if name.is_empty() {
            return;
        }
        spawn(async move {
            if api::calendars::create(&CreateCalendarDashboardInput { name, description: None })
                .await
                .is_ok()
            {
                new_cal_name.set(String::new());
                creating_cal.set(false);
                state.invalidate_calendars();
            }
        });
    };

    let items = match dashboards.read().as_ref() {
        Some(Ok(v)) => v.clone(),
        _ => Vec::new(),
    };
    let wr_items = match war_rooms.read().as_ref() {
        Some(Ok(v)) => v.clone(),
        _ => Vec::new(),
    };
    let cal_items = match calendars.read().as_ref() {
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

                div { class: "sidebar-section", "War Rooms" }
                for w in wr_items {
                    Link {
                        key: "{w.id}",
                        to: Route::WarRoomPage { id: w.id },
                        class: if active_wr == Some(w.id) { "nav-item active" } else { "nav-item" },
                        Icon { width: 16, height: 16, icon: LdSiren }
                        "{w.name}"
                    }
                }
                if creating_wr() {
                    input {
                        class: "input",
                        style: "margin-top:4px;",
                        autofocus: true,
                        placeholder: "War room name…",
                        value: "{new_wr_name}",
                        oninput: move |e| new_wr_name.set(e.value()),
                        onblur: move |_| creating_wr.set(false),
                        onkeydown: move |e| {
                            if e.key() == Key::Enter {
                                submit_create_wr(());
                            } else if e.key() == Key::Escape {
                                creating_wr.set(false);
                            }
                        },
                    }
                } else {
                    button {
                        class: "nav-item subtle",
                        onclick: move |_| creating_wr.set(true),
                        Icon { width: 16, height: 16, icon: LdPlus }
                        "New war room"
                    }
                }

                div { class: "sidebar-section", "Calendar" }
                for c in cal_items {
                    Link {
                        key: "{c.id}",
                        to: Route::CalendarPage { id: c.id },
                        class: if active_cal == Some(c.id) { "nav-item active" } else { "nav-item" },
                        Icon { width: 16, height: 16, icon: LdCalendar }
                        "{c.name}"
                    }
                }
                if creating_cal() {
                    input {
                        class: "input",
                        style: "margin-top:4px;",
                        autofocus: true,
                        placeholder: "Calendar name…",
                        value: "{new_cal_name}",
                        oninput: move |e| new_cal_name.set(e.value()),
                        onblur: move |_| creating_cal.set(false),
                        onkeydown: move |e| {
                            if e.key() == Key::Enter {
                                submit_create_cal(());
                            } else if e.key() == Key::Escape {
                                creating_cal.set(false);
                            }
                        },
                    }
                } else {
                    button {
                        class: "nav-item subtle",
                        onclick: move |_| creating_cal.set(true),
                        Icon { width: 16, height: 16, icon: LdPlus }
                        "New calendar"
                    }
                }

                div { class: "sidebar-section", "Tools" }
                Link { to: Route::NotesPage {}, class: "nav-item",
                    Icon { width: 16, height: 16, icon: LdStickyNote }
                    "Notes"
                }
                button {
                    class: "nav-item subtle",
                    onclick: move |_| show_repos.set(true),
                    Icon { width: 16, height: 16, icon: LdFolderGit2 }
                    "Repos"
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
        if show_repos() {
            ReposDialog { on_close: move |_| show_repos.set(false) }
        }
    }
}
