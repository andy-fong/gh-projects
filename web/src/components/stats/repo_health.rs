//! Per-repo PR health.

use dioxus::prelude::*;

use crate::types::RepoHealth;

fn hours_label(h: Option<f64>) -> String {
    match h {
        None => "—".into(),
        Some(h) if h < 48.0 => format!("{h:.1}h"),
        Some(h) => format!("{:.1}d", h / 24.0),
    }
}

#[component]
fn MicroBar(pct: f64, tone: String) -> Element {
    rsx! {
        div { class: "ts-microbar",
            span { class: "ts-microbar-label mono", "{pct:.0}%" }
            div { class: "ts-bar",
                div { class: "ts-bar-fill {tone}", style: "width:{pct.clamp(0.0, 100.0):.0}%;" }
            }
        }
    }
}

#[component]
pub fn RepoHealthTable(rows: Vec<RepoHealth>) -> Element {
    if rows.is_empty() {
        return rsx! { div { class: "empty-hint", "No tracked repos have data yet." } };
    }
    rsx! {
        div { class: "ts-table-wrap",
            table { class: "gh-table ts-table",
                thead {
                    tr {
                        th { "Repo" }
                        th { class: "num", "Open" }
                        th { class: "num", title: "Open, non-draft, no human review", "Unreviewed" }
                        th { class: "num", "> 7d" }
                        th { class: "num", title: "Oldest unreviewed open PR", "Oldest" }
                        th { class: "num", "Merged" }
                        th {
                            class: "num",
                            title: "Merged with nobody having approved it",
                            "No approval"
                        }
                        th {
                            class: "num",
                            title: "Open PRs carrying the stale label right now, and how many the bot closed in this window",
                            "Stale"
                        }
                        th { class: "num", title: "Median time to first human review", "TTFR med" }
                        th { class: "num", title: "90th percentile — the tail that actually hurts", "p90" }
                        th { title: "Share of PRs opened in-window that never got a human review", "Never reviewed" }
                    }
                }
                tbody {
                    for r in rows.iter() {
                        tr { key: "{r.repo}",
                            td { title: "{r.repo}", "{r.repo.split('/').last().unwrap_or(&r.repo)}" }
                            td { class: "num", "{r.open_total}" }
                            td { class: "num",
                                span {
                                    class: if r.open_unreviewed > 0 { "ts-ratio-value warn" } else { "" },
                                    "{r.open_unreviewed}"
                                }
                            }
                            td { class: "num",
                                span {
                                    class: if r.open_unreviewed_7d > 0 { "ts-ratio-value bad" } else { "" },
                                    "{r.open_unreviewed_7d}"
                                }
                            }
                            td { class: "num muted",
                                match r.oldest_open_unreviewed_days {
                                    Some(d) => rsx! { "{d}d" },
                                    None => rsx! { "—" },
                                }
                            }
                            td { class: "num", "{r.merged_in_window}" }
                            td { class: "num",
                                span {
                                    class: if r.merged_without_approval > 0 { "ts-ratio-value bad" } else { "muted" },
                                    "{r.merged_without_approval}"
                                }
                            }
                            td { class: "num",
                                if r.stale_open_now == 0 && r.stale_closed == 0 {
                                    span { class: "muted", "—" }
                                } else {
                                    span {
                                        class: if r.stale_open_now > 0 { "ts-ratio-value warn" } else { "muted" },
                                        "{r.stale_open_now}"
                                    }
                                    if r.stale_closed > 0 {
                                        span {
                                            class: "muted text-xs",
                                            title: "closed by the bot in this window",
                                            " (+{r.stale_closed} closed)"
                                        }
                                    }
                                }
                            }
                            td { class: "num mono", "{hours_label(r.median_ttfr_hours)}" }
                            td { class: "num mono",
                                span {
                                    class: match r.p90_ttfr_hours {
                                        Some(h) if h > 168.0 => "ts-ratio-value bad",
                                        Some(h) if h > 48.0 => "ts-ratio-value warn",
                                        _ => "",
                                    },
                                    "{hours_label(r.p90_ttfr_hours)}"
                                }
                            }
                            td {
                                match r.pct_never_reviewed {
                                    Some(p) => rsx! {
                                        MicroBar {
                                            pct: p,
                                            tone: if p >= 25.0 { "bad".to_string() }
                                                  else if p >= 10.0 { "warn".to_string() }
                                                  else { "neutral".to_string() },
                                        }
                                    },
                                    None => rsx! { span { class: "muted", "—" } },
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
