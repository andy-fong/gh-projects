//! Team Stats — PR authoring vs. review debt.
//!
//! Ordered diagnosis -> explanation -> action: KPI strip (how bad), scoreboard
//! (who), trend (getting better?), repo health (where), worklist (what to do).

use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{
    LdArrowDownWideNarrow, LdArrowUpNarrowWide, LdChevronDown, LdLineChart,
};
use dioxus_free_icons::Icon;
use std::collections::{BTreeMap, BTreeSet};

use crate::api;
use crate::components::detail_panel::DetailPanel;
use crate::components::stats::chart::{ChartSeries, TrendChart};
use crate::components::stats::kpi_strip::KpiStrip;
use crate::components::stats::repo_health::RepoHealthTable;
use crate::components::stats::scoreboard::Scoreboard;
use crate::components::stats::sync_bar::SyncBar;
use crate::components::stats::unreviewed_list::UnreviewedList;
use crate::state::use_app_state;
use crate::types::*;

const WEEK_CHOICES: [i64; 5] = [1, 4, 8, 12, 26];
const GROUP_CHOICES: [(&str, &str); 4] =
    [("all", "All"), ("team", "Team"), ("pe", "PE"), ("solo", "Solo")];

#[component]
pub fn TeamStatsPage() -> Element {
    let state = use_app_state();

    let mut weeks = use_signal(|| 8i64);
    // Default to `team`: PE, maintainers and community do a different job and
    // scoring them on the same ratio is what makes the table arguable.
    let mut group = use_signal(|| "team".to_string());
    let mut repo_filter = use_signal(BTreeSet::<String>::new);
    let mut show_repo_picker = use_signal(|| false);
    let mut worklist_filter = use_signal(|| "unreviewed".to_string());
    let mut worklist_sort = use_signal(|| "oldest".to_string());
    let mut detail = use_signal(|| None::<WorklistRow>);

    // Tracked-repo registry, for the filter picker and the empty state.
    let repos = use_resource(move || {
        let _ = state.stats_ver.read();
        let _ = state.repos_ver.read();
        async move { api::team_stats::repos().await }
    });

    // Every reactive read must happen in the synchronous prologue — signals
    // read inside `async move` do not register as dependencies.
    let summary = use_resource(move || {
        let _ = state.stats_ver.read();
        let w = weeks();
        let g = group();
        let r: Vec<String> = repo_filter().iter().cloned().collect();
        async move { api::team_stats::summary(w, &g, &r).await }
    });

    let worklist = use_resource(move || {
        let _ = state.stats_ver.read();
        let f = worklist_filter();
        let so = worklist_sort();
        let r: Vec<String> = repo_filter().iter().cloned().collect();
        async move { api::team_stats::worklist(&f, &so, &r).await }
    });

    let tracked: Vec<TrackedRepoStatus> = repos
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|v| v.iter().filter(|r| r.track_stats).cloned().collect())
        .unwrap_or_default();
    let any_tracked = !tracked.is_empty();
    let all_repos: Vec<TrackedRepoStatus> = repos
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .cloned()
        .unwrap_or_default();

    let summary_read = summary.read();
    let data: Option<&StatsSummary> = summary_read.as_ref().and_then(|r| r.as_ref().ok());
    let load_error: Option<String> = summary_read
        .as_ref()
        .and_then(|r| r.as_ref().err())
        .cloned();

    rsx! {
        div { class: "ts-page",
            div { class: "ts-header",
                div { class: "ts-controls",
                    h1 { class: "page-title",
                        Icon { width: 18, height: 18, icon: LdLineChart }
                        "Team Stats"
                    }

                    div { class: "ts-control-group",
                        for w in WEEK_CHOICES.iter().copied() {
                            button {
                                key: "{w}",
                                class: if weeks() == w { "chip active" } else { "chip" },
                                onclick: move |_| weeks.set(w),
                                "{w}w"
                            }
                        }
                    }

                    div { class: "ts-control-group",
                        for (val, label) in GROUP_CHOICES.iter() {
                            button {
                                key: "{val}",
                                class: if group() == *val { "chip active" } else { "chip" },
                                onclick: move |_| group.set(val.to_string()),
                                "{label}"
                            }
                        }
                    }

                    div { class: "ts-repo-picker",
                        button {
                            class: if repo_filter().is_empty() { "chip" } else { "chip active" },
                            onclick: move |_| show_repo_picker.toggle(),
                            match repo_filter().len() {
                                0 => rsx! { "All repos" },
                                // With one selected, name it — "1 repo" makes you
                                // open the menu again just to see which.
                                1 => {
                                    let only = repo_filter().iter().next().cloned().unwrap_or_default();
                                    let short = only.split('/').last().unwrap_or(&only).to_string();
                                    rsx! { "{short}" }
                                }
                                n => rsx! { "{n} repos" },
                            }
                            Icon { width: 12, height: 12, icon: LdChevronDown }
                        }
                        if show_repo_picker() {
                            div { class: "popover popover-menu ts-repo-menu",
                                button {
                                    class: "popover-item",
                                    onclick: move |_| repo_filter.write().clear(),
                                    "All repos"
                                }
                                for r in tracked.iter() {
                                    {
                                        let key = r.owner_repo.to_lowercase();
                                        let on = repo_filter().contains(&key);
                                        rsx! {
                                            button {
                                                key: "{r.owner_repo}",
                                                class: if on { "popover-item active" } else { "popover-item" },
                                                onclick: move |_| {
                                                    let mut f = repo_filter.write();
                                                    if !f.remove(&key) { f.insert(key.clone()); }
                                                },
                                                "{r.owner_repo}"
                                                span { class: "muted text-xs", " {r.pr_count}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div { class: "spacer" }

                    SyncBar {
                        last_synced: data.and_then(|d| d.last_synced_at.clone()),
                        tracked_count: tracked.len(),
                    }
                }
            }

            if let Some(e) = load_error {
                div { class: "error-text", "{e}" }
            }

            // Nobody would guess they have to open the Repos dialog first, so
            // say it plainly rather than rendering five empty tables.
            if !any_tracked && !all_repos.is_empty() {
                div { class: "ts-section",
                    div { class: "ts-empty",
                        h3 { "No repos are tracked yet" }
                        p { class: "muted",
                            "Team Stats only syncs repos you opt in. Open "
                            b { "Repos" }
                            " in the sidebar and tick \"Track stats\" on the ones your team works in."
                        }
                        p { class: "hint",
                            "Leave very busy upstream repos off — they dominate the numbers without telling you much about your team."
                        }
                    }
                }
            }

            if let Some(d) = data {
                KpiStrip { totals: d.totals.clone() }

                div { class: "ts-section",
                    div { class: "ts-section-head",
                        "Review debt"
                        span { class: "spacer" }
                        span {
                            class: "muted text-xs",
                            title: "The current, still-running week is excluded so a partial week can't drag the numbers down",
                            {
                                let wk = if d.weeks == 1 { "full week" } else { "full weeks" };
                                let ppl = if d.people.len() == 1 { "person" } else { "people" };
                                format!("{} {} · {} → {} · {} {}",
                                    d.weeks, wk, d.since, d.through, d.people.len(), ppl)
                            }
                        }
                    }
                    div { class: "ts-section-body",
                        Scoreboard {
                            rows: d.people.clone(),
                            spark: spark_map(d),
                        }
                    }
                }

                div { class: "ts-section",
                    div { class: "ts-section-head", "Weekly trend" }
                    div { class: "ts-section-body",
                        TrendChart {
                            weeks: d.trends.iter().map(|w| w.week_start.clone()).collect(),
                            series: vec![
                                ChartSeries {
                                    name: "Opened".into(),
                                    color: "var(--accent)".into(),
                                    values: d.trends.iter().map(|w| w.prs_opened as f64).collect(),
                                },
                                ChartSeries {
                                    name: "Reviewed".into(),
                                    color: "var(--good)".into(),
                                    values: d.trends.iter().map(|w| w.prs_reviewed as f64).collect(),
                                },
                                ChartSeries {
                                    name: "AI reviews".into(),
                                    color: "var(--warn)".into(),
                                    values: d.trends.iter().map(|w| w.bot_review_events as f64).collect(),
                                },
                                ChartSeries {
                                    name: "Merged w/o review".into(),
                                    color: "var(--danger)".into(),
                                    values: d.trends.iter().map(|w| w.prs_merged_without_review as f64).collect(),
                                },
                            ],
                            height: 200,
                        }
                    }
                }

                div { class: "ts-section",
                    div { class: "ts-section-head", "PR health by repo" }
                    div { class: "ts-section-body",
                        RepoHealthTable { rows: d.repo_health.clone() }
                    }
                }
            }

            div { class: "ts-section",
                div { class: "ts-section-head",
                    "Needs a reviewer"
                    span { class: "spacer" }
                    div { class: "ts-control-group",
                        for (val, label) in [("unreviewed", "No human review"), ("bot_only", "AI only"), ("all", "All open")].iter() {
                            button {
                                key: "{val}",
                                class: if worklist_filter() == *val { "chip active" } else { "chip" },
                                onclick: move |_| worklist_filter.set(val.to_string()),
                                "{label}"
                            }
                        }
                    }
                    button {
                        class: "chip ts-sort-toggle",
                        title: "Flip the ordering",
                        onclick: move |_| {
                            let next = if worklist_sort() == "oldest" { "newest" } else { "oldest" };
                            worklist_sort.set(next.to_string());
                        },
                        if worklist_sort() == "oldest" {
                            Icon { width: 12, height: 12, icon: LdArrowDownWideNarrow }
                            "Oldest first"
                        } else {
                            Icon { width: 12, height: 12, icon: LdArrowUpNarrowWide }
                            "Newest first"
                        }
                    }
                }
                div { class: "ts-section-body",
                    match worklist.read().as_ref() {
                        Some(Ok(rows)) => rsx! {
                            UnreviewedList {
                                rows: rows.clone(),
                                on_open: move |r: WorklistRow| detail.set(Some(r)),
                            }
                        },
                        Some(Err(e)) => rsx! { div { class: "error-text", "{e}" } },
                        None => rsx! { div { class: "loading-text", "Loading…" } },
                    }
                }
            }
        }

        if let Some(r) = detail() {
            DetailPanel {
                repo: r.repo.clone(),
                ref_type: "pr".to_string(),
                number: r.number,
                url: r.url.clone(),
                on_close: move |_| detail.set(None),
            }
        }
    }
}

/// Per-member weekly "distinct PRs reviewed", for the scoreboard sparklines.
fn spark_map(d: &StatsSummary) -> BTreeMap<String, Vec<f64>> {
    d.member_trends
        .iter()
        .map(|m| {
            (
                m.login.to_lowercase(),
                m.reviewed.iter().map(|v| *v as f64).collect(),
            )
        })
        .collect()
}
