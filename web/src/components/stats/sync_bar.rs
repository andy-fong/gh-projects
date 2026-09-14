//! Freshness readout plus the manual sync button.

use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{LdRefreshCw, LdTriangleAlert, LdX};
use dioxus_free_icons::Icon;

use crate::api;
use crate::datetime::relative_from_now;
use crate::state::use_app_state;
use crate::types::SyncResult;

#[component]
pub fn SyncBar(last_synced: Option<String>, tracked_count: usize) -> Element {
    let state = use_app_state();
    let mut pending = use_signal(|| false);
    let mut result = use_signal(|| None::<SyncResult>);
    let mut error = use_signal(|| None::<String>);
    let mut show_failures = use_signal(|| false);

    // Stale enough that the numbers on screen should not be trusted silently.
    let stale = last_synced
        .as_deref()
        .map(|t| relative_from_now(t).ends_with("d ago") || relative_from_now(t).contains("mo"))
        .unwrap_or(true);

    let run = move |_| {
        pending.set(true);
        error.set(None);
        show_failures.set(false);
        spawn(async move {
            match api::team_stats::sync().await {
                Ok(r) => {
                    result.set(Some(r));
                    state.invalidate_stats();
                }
                Err(e) => error.set(Some(e)),
            }
            pending.set(false);
        });
    };

    rsx! {
        div { class: "ts-sync",
            span {
                class: if stale { "ts-sync-freshness stale" } else { "ts-sync-freshness" },
                match last_synced.as_deref() {
                    Some(t) => rsx! { "Synced {relative_from_now(t)}" },
                    None => rsx! { "Never synced" },
                }
            }
            button {
                class: "btn btn-sm btn-primary",
                disabled: pending(),
                onclick: run,
                span { class: if pending() { "spin" } else { "" },
                    Icon { width: 14, height: 14, icon: LdRefreshCw }
                }
                if pending() { "Syncing…" } else { "Sync now" }
            }
        }

        if pending() {
            div { class: "hint",
                "Walking {tracked_count} repos — a first backfill takes about a minute."
            }
        }

        if let Some(r) = result() {
            div { class: "ts-sync-result",
                span {
                    "Synced {r.repos.len()} repos · {r.counts.prs} PRs · {r.counts.reviews} reviews · {r.rate_limit.cost_total} API points"
                }
                // Partial failure is the normal case (a repo 403s, gets
                // archived, rate-limits), so name which repo and why rather
                // than failing the whole run.
                if !r.errors.is_empty() {
                    button {
                        class: "chip warn",
                        onclick: move |_| show_failures.toggle(),
                        Icon { width: 12, height: 12, icon: LdTriangleAlert }
                        "{r.errors.len()} of {r.repos.len()} failed"
                    }
                }
                button {
                    class: "icon-btn",
                    onclick: move |_| result.set(None),
                    Icon { width: 12, height: 12, icon: LdX }
                }
            }
            if show_failures() {
                div { class: "ts-sync-failures",
                    for e in r.errors.iter() {
                        div { class: "ts-sync-failure error-text", "{e}" }
                    }
                }
            }
        }

        if let Some(e) = error() {
            div { class: "error-text", "{e}" }
        }
    }
}
