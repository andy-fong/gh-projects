use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{LdCheck, LdChevronDown, LdChevronUp, LdCopy, LdPlus, LdTag, LdX};
use dioxus_free_icons::Icon;

use crate::api;
use crate::platform;
use crate::state::use_app_state;
use crate::types::{CreateReleaseWatchInput, ReleaseWatch};

/// Returns a (major, minor, patch, is_release) tuple suitable for descending semver sort.
/// Strips a leading `v`/`V`, ignores build metadata, treats pre-release as lower than release.
fn semver_key(tag: &str) -> (u64, u64, u64, bool) {
    let s = tag.trim_start_matches(|c: char| c == 'v' || c == 'V');
    let (version, is_pre) = match s.find(|c: char| c == '-' || c == '+') {
        Some(i) => (&s[..i], true),
        None => (s, false),
    };
    let mut parts = version.splitn(4, '.');
    let major = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0u64);
    let minor = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0u64);
    let patch = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0u64);
    (major, minor, patch, !is_pre)
}

#[component]
pub fn ReleasesPanel(on_close: EventHandler<()>) -> Element {
    let state = use_app_state();
    let mut adding = use_signal(|| false);
    let mut new_repo_id: Signal<Option<i64>> = use_signal(|| None);
    let mut new_limit: Signal<u32> = use_signal(|| 4);

    let watches_res = use_resource(move || {
        let _ = state.release_watches_ver.read();
        async move { api::release_watches::list().await }
    });

    let repos_res = use_resource(move || {
        let _ = state.repos_ver.read();
        async move { api::repos::list().await }
    });

    let watches = use_memo(move || -> Vec<ReleaseWatch> {
        match watches_res.read().as_ref() {
            Some(Ok(v)) => v.clone(),
            _ => Vec::new(),
        }
    });

    let repo_list = match repos_res.read().as_ref() {
        Some(Ok(v)) => v.clone(),
        _ => Vec::new(),
    };

    let add_watch = move |_| {
        let Some(repo_id) = new_repo_id() else { return };
        let lim = new_limit() as i64;
        spawn(async move {
            if api::release_watches::create(&CreateReleaseWatchInput {
                repo_id,
                limit_count: lim,
            })
            .await
            .is_ok()
            {
                state.invalidate_release_watches();
            }
        });
        adding.set(false);
        new_repo_id.set(None);
        new_limit.set(4);
    };

    let remove_watch = move |id: i64| {
        spawn(async move {
            if api::release_watches::delete(id).await.is_ok() {
                state.invalidate_release_watches();
            }
        });
    };

    let move_watch_up = move |id: i64| {
        let mut ws = watches();
        if let Some(idx) = ws.iter().position(|w| w.id == id) {
            if idx > 0 {
                ws.swap(idx, idx - 1);
                let ids: Vec<i64> = ws.iter().map(|w| w.id).collect();
                spawn(async move {
                    if api::release_watches::reorder(&ids).await.is_ok() {
                        state.invalidate_release_watches();
                    }
                });
            }
        }
    };

    let move_watch_down = move |id: i64| {
        let mut ws = watches();
        if let Some(idx) = ws.iter().position(|w| w.id == id) {
            if idx + 1 < ws.len() {
                ws.swap(idx, idx + 1);
                let ids: Vec<i64> = ws.iter().map(|w| w.id).collect();
                spawn(async move {
                    if api::release_watches::reorder(&ids).await.is_ok() {
                        state.invalidate_release_watches();
                    }
                });
            }
        }
    };

    let watch_list = watches();
    let watch_count = watch_list.len();

    rsx! {
        div { class: "releases-panel",
            div { class: "releases-panel-head",
                div { class: "flex items-center gap-2",
                    Icon { width: 15, height: 15, icon: LdTag }
                    span { style: "font-weight:600; font-size:14px;", "Releases" }
                }
                button {
                    class: "icon-btn",
                    title: "Close panel",
                    onclick: move |_| on_close.call(()),
                    Icon { width: 15, height: 15, icon: LdX }
                }
            }

            div { class: "releases-panel-body",
                if adding() {
                    div { class: "releases-add-form",
                        select {
                            class: "select",
                            style: "font-size:13px; padding:6px 8px;",
                            onchange: move |e| {
                                let val = e.value();
                                if val.is_empty() {
                                    new_repo_id.set(None);
                                } else if let Ok(id) = val.parse::<i64>() {
                                    new_repo_id.set(Some(id));
                                }
                            },
                            option { value: "", "— pick a repo —" }
                            for r in repo_list.iter() {
                                option {
                                    key: "{r.id}",
                                    value: "{r.id}",
                                    selected: new_repo_id() == Some(r.id),
                                    "{r.name}"
                                }
                            }
                        }
                        div { class: "flex gap-2 items-center",
                            label { style: "font-size:12px; color:var(--muted); flex-shrink:0;", "Limit" }
                            input {
                                class: "input",
                                style: "font-size:13px; padding:5px 8px;",
                                r#type: "number",
                                min: "1",
                                max: "20",
                                value: "{new_limit}",
                                oninput: move |e| {
                                    if let Ok(n) = e.value().parse::<u32>() {
                                        if n >= 1 && n <= 20 {
                                            new_limit.set(n);
                                        }
                                    }
                                },
                            }
                        }
                        div { class: "flex gap-2",
                            button {
                                class: "btn btn-primary btn-sm",
                                style: "flex:1; justify-content:center;",
                                disabled: new_repo_id().is_none(),
                                onclick: add_watch,
                                "Add"
                            }
                            button {
                                class: "btn btn-sm",
                                onclick: move |_| {
                                    adding.set(false);
                                    new_repo_id.set(None);
                                    new_limit.set(4);
                                },
                                "Cancel"
                            }
                        }
                    }
                } else {
                    button {
                        class: "releases-add-btn",
                        onclick: move |_| adding.set(true),
                        Icon { width: 14, height: 14, icon: LdPlus }
                        "Add repo"
                    }
                }

                for (idx, watch) in watch_list.into_iter().enumerate() {
                    {
                        let watch_id = watch.id;
                        let is_first = idx == 0;
                        let is_last = idx + 1 == watch_count;
                        rsx! {
                            ReleaseWatchItem {
                                key: "{watch_id}",
                                watch: watch.clone(),
                                is_first,
                                is_last,
                                on_remove: move |_| remove_watch(watch_id),
                                on_move_up: move |_| move_watch_up(watch_id),
                                on_move_down: move |_| move_watch_down(watch_id),
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ReleaseWatchItem(
    watch: ReleaseWatch,
    is_first: bool,
    is_last: bool,
    on_move_up: EventHandler<()>,
    on_move_down: EventHandler<()>,
    on_remove: EventHandler<()>,
) -> Element {
    let state = use_app_state();
    let mut last_copied: Signal<Option<String>> = use_signal(|| None);

    let repo_id = watch.repo_id;
    let limit = watch.limit_count;

    let repos_res = use_resource(move || {
        let _ = state.repos_ver.read();
        async move { api::repos::list().await }
    });

    // Reactive: recomputes when repos finish loading.
    let owner_repo = use_memo(move || -> Option<String> {
        match repos_res.read().as_ref() {
            Some(Ok(list)) => list.iter().find(|r| r.id == repo_id).map(|r| r.owner_repo.clone()),
            _ => None,
        }
    });

    let repo_name = use_memo(move || -> String {
        match repos_res.read().as_ref() {
            Some(Ok(list)) => list
                .iter()
                .find(|r| r.id == repo_id)
                .map(|r| r.name.clone())
                .unwrap_or_default(),
            _ => String::new(),
        }
    });

    // Reactive: re-runs when owner_repo changes or the gh cache is invalidated.
    let releases = use_resource(move || {
        let _ = state.gh_ver.read();
        let or = owner_repo();
        async move {
            let owner_repo = or?;
            Some(
                api::gh::execute(&format!(
                    "release list --repo {owner_repo} --limit {limit} --json tagName"
                ))
                .await,
            )
        }
    });

    let repos_loaded = matches!(repos_res.read().as_ref(), Some(_));
    let or = owner_repo();
    let is_loading = !repos_loaded || (or.is_some() && releases.read().is_none());
    let repo_not_found = repos_loaded && or.is_none();

    let tag_names: Vec<String> = {
        let mut tags: Vec<String> = match releases.read().as_ref() {
            Some(Some(Ok(resp))) => resp
                .output
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.get("tagName").and_then(|t| t.as_str()).map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        tags.sort_by(|a, b| semver_key(b).cmp(&semver_key(a)));
        tags
    };

    let gh_error: Option<String> = match releases.read().as_ref() {
        Some(Some(Err(e))) => Some(e.clone()),
        _ => None,
    };

    rsx! {
        div { class: "release-watch-block",
            div { class: "release-watch-head",
                div { class: "flex items-center gap-2", style: "min-width:0; flex:1;",
                    span { class: "release-watch-name", "{repo_name}" }
                    span { class: "release-watch-limit", "×{limit}" }
                }
                button {
                    class: "icon-btn",
                    title: "Move up",
                    disabled: is_first,
                    onclick: move |_| on_move_up.call(()),
                    Icon { width: 13, height: 13, icon: LdChevronUp }
                }
                button {
                    class: "icon-btn",
                    title: "Move down",
                    disabled: is_last,
                    onclick: move |_| on_move_down.call(()),
                    Icon { width: 13, height: 13, icon: LdChevronDown }
                }
                button {
                    class: "icon-btn",
                    title: "Remove",
                    onclick: move |_| on_remove.call(()),
                    Icon { width: 13, height: 13, icon: LdX }
                }
            }
            div {
                if repo_not_found {
                    div { class: "releases-empty", "Repo not found in registry" }
                } else if is_loading {
                    div { class: "loading-text", style: "padding:8px 12px; font-size:12px;", "Loading…" }
                } else if let Some(err) = gh_error {
                    div { class: "error-text", style: "padding:8px 12px; font-size:12px;", "{err}" }
                } else if tag_names.is_empty() {
                    div { class: "releases-empty", "No releases found" }
                } else {
                    for tag in tag_names.iter() {
                        div { class: "release-row",
                            key: "{tag}",
                            span { class: "release-tag mono", "{tag}" }
                            button {
                                class: "icon-btn",
                                title: "Copy",
                                onclick: {
                                    let tag = tag.clone();
                                    move |_| {
                                        platform::copy_to_clipboard(&tag);
                                        last_copied.set(Some(tag.clone()));
                                    }
                                },
                                if last_copied().as_deref() == Some(tag.as_str()) {
                                    Icon { width: 13, height: 13, icon: LdCheck }
                                } else {
                                    Icon { width: 13, height: 13, icon: LdCopy }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
