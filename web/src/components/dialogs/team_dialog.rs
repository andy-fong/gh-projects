use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{LdPlus, LdRefreshCcw, LdTrash2, LdX};
use dioxus_free_icons::Icon;

use crate::api;
use crate::state::use_app_state;
use crate::types::{
    CreateTeamMemberInput, CreateTeamMemberSourceInput, TeamMember, UpdateTeamMemberInput,
};

/// The buckets a login can be filed under, with their display labels. Anyone
/// absent from the roster is classified `community`, so it isn't listed here.
const GROUPS: [(&str, &str); 5] = [
    ("team", "Team"),
    ("pe", "PE"),
    ("solo", "Solo"),
    ("maintainer", "Maintainer"),
    ("bot", "Bot"),
];

fn group_label(group: &str) -> &'static str {
    GROUPS
        .iter()
        .find(|(g, _)| *g == group)
        .map(|(_, l)| *l)
        .unwrap_or("Team")
}

/// Manage the roster of GitHub logins that drives the `authorGroup` column in
/// GH Query tiles. Logins can be typed in or imported from a GitHub team/org.
#[component]
pub fn TeamDialog(on_close: EventHandler<()>) -> Element {
    let state = use_app_state();

    let members = use_resource(move || {
        let _ = state.team_ver.read();
        async move { api::team_members::list().await }
    });
    let sources = use_resource(move || {
        let _ = state.team_ver.read();
        async move { api::team_members::list_sources().await }
    });

    let mut new_login = use_signal(String::new);
    let mut new_group = use_signal(|| "team".to_string());
    let mut new_source = use_signal(String::new);
    let mut new_source_group = use_signal(|| "maintainer".to_string());
    let mut refreshing = use_signal(|| false);
    let mut refresh_msg = use_signal(String::new);

    let add_member = move || {
        let login = new_login.read().trim().to_string();
        if login.is_empty() {
            return;
        }
        let member_group = new_group.read().clone();
        let input = CreateTeamMemberInput { login, member_group };
        spawn(async move {
            match api::team_members::create(&input).await {
                Ok(_) => {
                    new_login.set(String::new());
                    refresh_msg.set(String::new());
                    state.invalidate_team();
                }
                Err(e) => refresh_msg.set(e),
            }
        });
    };

    let add_source = move || {
        let org_team = new_source.read().trim().to_string();
        if org_team.is_empty() {
            return;
        }
        let member_group = new_source_group.read().clone();
        spawn(async move {
            match api::team_members::create_source(&CreateTeamMemberSourceInput {
                member_group,
                org_team,
            })
            .await
            {
                Ok(_) => {
                    new_source.set(String::new());
                    refresh_msg.set(String::new());
                    state.invalidate_team();
                }
                Err(e) => refresh_msg.set(e),
            }
        });
    };

    let refresh = move |_| {
        if refreshing() {
            return;
        }
        refreshing.set(true);
        refresh_msg.set(String::new());
        spawn(async move {
            match api::team_members::refresh().await {
                Ok(r) => {
                    let mut msg = format!("Added {}, skipped {}.", r.added, r.skipped);
                    if !r.errors.is_empty() {
                        msg.push(' ');
                        msg.push_str(&r.errors.join("; "));
                    }
                    refresh_msg.set(msg);
                    state.invalidate_team();
                }
                Err(e) => refresh_msg.set(e),
            }
            refreshing.set(false);
        });
    };

    let all: Vec<TeamMember> = match members.read().as_ref() {
        Some(Ok(v)) => v.clone(),
        _ => Vec::new(),
    };
    let src_items = match sources.read().as_ref() {
        Some(Ok(v)) => v.clone(),
        _ => Vec::new(),
    };

    rsx! {
        div { class: "modal-backdrop", onclick: move |_| on_close.call(()),
            div {
                class: "modal",
                style: "width: 640px; max-height: 82vh; overflow-y: auto;",
                onclick: move |e| e.stop_propagation(),
                div { class: "modal-header",
                    h2 { class: "modal-title", "Team" }
                    button { class: "icon-btn", onclick: move |_| on_close.call(()),
                        Icon { width: 16, height: 16, icon: LdX }
                    }
                }
                div { class: "hint", style: "margin-bottom:12px;",
                    "GH Query tiles tag every row with an "
                    code { "authorGroup" }
                    " column. Logins listed here become team / pe / solo / maintainer / bot; everyone else is community."
                }

                for (group , label) in GROUPS.iter() {
                    {
                        let rows: Vec<TeamMember> = all.iter().filter(|m| m.member_group == *group).cloned().collect();
                        rsx! {
                            div { key: "{group}", style: "margin-bottom:14px;",
                                div { class: "sidebar-section", style: "padding-left:0;",
                                    "{label} ({rows.len()})"
                                }
                                if rows.is_empty() {
                                    div { class: "muted text-xs", style: "margin-bottom:8px;", "None yet" }
                                }
                                for m in rows {
                                    TeamMemberRow {
                                        key: "{m.id}",
                                        id: m.id,
                                        login: m.login.clone(),
                                        member_group: m.member_group.clone(),
                                        source: m.source.clone(),
                                    }
                                }
                            }
                        }
                    }
                }

                div { class: "wr-repo-row",
                    input {
                        class: "input",
                        placeholder: "GitHub login",
                        value: "{new_login}",
                        oninput: move |e| new_login.set(e.value()),
                        onkeydown: {
                            let a = add_member;
                            move |e: Event<KeyboardData>| if e.key() == Key::Enter { a() }
                        },
                    }
                    select {
                        class: "select",
                        onchange: move |e| new_group.set(e.value()),
                        for (g , label) in GROUPS.iter() {
                            option {
                                value: "{g}",
                                selected: new_group.read().as_str() == *g,
                                "{label}"
                            }
                        }
                    }
                    button {
                        class: "btn btn-primary btn-sm",
                        onclick: {
                            let a = add_member;
                            move |_| a()
                        },
                        Icon { width: 14, height: 14, icon: LdPlus }
                        "Add"
                    }
                }

                div { class: "sidebar-section", style: "padding-left:0; margin-top:18px;", "Import sources" }
                div { class: "hint", style: "margin-bottom:8px;",
                    "A GitHub team as "
                    code { "owner/team-slug" }
                    ", or a whole org as just "
                    code { "owner" }
                    ". Refresh adds any missing logins; existing ones keep the group they already have."
                }
                for s in src_items {
                    div { key: "{s.id}", class: "wr-repo-row",
                        span { class: "text-xs", style: "flex:1;", "{s.org_team}" }
                        span { class: "muted text-xs", style: "flex:0 0 90px;", "{group_label(&s.member_group)}" }
                        button {
                            class: "icon-btn danger",
                            title: "Remove source",
                            onclick: move |_| {
                                let id = s.id;
                                spawn(async move {
                                    if api::team_members::delete_source(id).await.is_ok() {
                                        state.invalidate_team();
                                    }
                                });
                            },
                            Icon { width: 16, height: 16, icon: LdTrash2 }
                        }
                    }
                }
                div { class: "wr-repo-row",
                    input {
                        class: "input",
                        placeholder: "kgateway-dev/controller-maintainers",
                        value: "{new_source}",
                        oninput: move |e| new_source.set(e.value()),
                        onkeydown: {
                            let a = add_source;
                            move |e: Event<KeyboardData>| if e.key() == Key::Enter { a() }
                        },
                    }
                    select {
                        class: "select",
                        onchange: move |e| new_source_group.set(e.value()),
                        for (g , label) in GROUPS.iter() {
                            option {
                                value: "{g}",
                                selected: new_source_group.read().as_str() == *g,
                                "{label}"
                            }
                        }
                    }
                    button {
                        class: "btn btn-primary btn-sm",
                        onclick: {
                            let a = add_source;
                            move |_| a()
                        },
                        Icon { width: 14, height: 14, icon: LdPlus }
                        "Add"
                    }
                }

                div { style: "display:flex; align-items:center; gap:10px; margin-top:14px;",
                    button {
                        class: "btn btn-sm",
                        disabled: refreshing(),
                        title: "Re-import every source from GitHub",
                        onclick: refresh,
                        span { class: if refreshing() { "spin" } else { "" },
                            Icon { width: 14, height: 14, icon: LdRefreshCcw }
                        }
                        if refreshing() { "Refreshing…" } else { "Refresh from GitHub" }
                    }
                    if !refresh_msg.read().is_empty() {
                        span { class: "muted text-xs", "{refresh_msg}" }
                    }
                }
            }
        }
    }
}

#[component]
fn TeamMemberRow(id: i64, login: String, member_group: String, source: String) -> Element {
    let state = use_app_state();
    let mut login_sig = use_signal(|| login.clone());
    let group_sig = use_signal(|| member_group.clone());

    // Editing the login and re-assigning the group both go through one PUT so
    // the row behaves like the repo registry rows (save on blur / on change).
    let save = move |login: String, group: String| {
        if login.trim().is_empty() {
            return;
        }
        spawn(async move {
            let _ = api::team_members::update(
                id,
                &UpdateTeamMemberInput {
                    login: Some(login.trim().to_string()),
                    member_group: Some(group),
                    position: None,
                },
            )
            .await;
            state.invalidate_team();
        });
    };

    let remove = move |_| {
        spawn(async move {
            if api::team_members::delete(id).await.is_ok() {
                state.invalidate_team();
            }
        });
    };

    rsx! {
        div { class: "wr-repo-row",
            input {
                class: "input",
                value: "{login_sig}",
                oninput: move |e| login_sig.set(e.value()),
                onblur: move |_| save(login_sig.read().clone(), group_sig.read().clone()),
            }
            select {
                class: "select",
                onchange: move |e| {
                    let mut group_sig = group_sig;
                    group_sig.set(e.value());
                    save(login_sig.read().clone(), e.value());
                },
                for (g , label) in GROUPS.iter() {
                    option {
                        value: "{g}",
                        selected: group_sig.read().as_str() == *g,
                        "{label}"
                    }
                }
            }
            span {
                class: "muted text-xs",
                style: "flex:0 0 150px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;",
                title: "{source}",
                if source == "manual" { "manual" } else { "{source}" }
            }
            button { class: "icon-btn danger", title: "Remove", onclick: remove,
                Icon { width: 16, height: 16, icon: LdTrash2 }
            }
        }
    }
}
