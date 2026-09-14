//! Per-person review debt — the headline table.

use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::LdInfo;
use dioxus_free_icons::Icon;
use std::collections::BTreeMap;

use crate::components::stats::chart::Sparkline;
use crate::types::PersonStat;

/// Series colour per roster group, matching the `.badge-*` palette so a row's
/// sparkline and its group badge agree.
fn group_color(group: &str) -> &'static str {
    match group {
        "team" => "#3fb950",
        "pe" => "#db61a2",
        "solo" => "#a371f7",
        "maintainer" => "#58a6ff",
        "bot" => "#8b949e",
        _ => "#e3b341",
    }
}

fn ratio_tone(r: f64) -> &'static str {
    if r < 0.5 {
        "bad"
    } else if r < 1.0 {
        "warn"
    } else {
        "good"
    }
}

#[component]
pub fn Scoreboard(
    rows: Vec<PersonStat>,
    /// login -> weekly "distinct PRs reviewed", for the sparkline column.
    spark: BTreeMap<String, Vec<f64>>,
) -> Element {
    // Declared before the early return: Dioxus hooks must run in the same
    // order on every render, so a `use_signal` after a conditional `return`
    // would corrupt hook state the first time this renders with rows.
    let mut show_defs = use_signal(|| false);

    if rows.is_empty() {
        return rsx! { div { class: "empty-hint", "No activity for this group in this window." } };
    }

    // One scale for every row, so the sparkline heights are comparable.
    let spark_max = spark
        .values()
        .flat_map(|v| v.iter().copied())
        .fold(1.0f64, f64::max);

    let tot_opened: i64 = rows.iter().map(|r| r.prs_opened).sum();
    let tot_draft: i64 = rows.iter().map(|r| r.prs_draft).sum();
    let tot_merged: i64 = rows.iter().map(|r| r.prs_merged).sum();
    let tot_reviewed: i64 = rows.iter().map(|r| r.prs_reviewed).sum();
    let tot_appr: i64 = rows.iter().map(|r| r.approvals).sum();
    let tot_merge_ratio = if tot_opened == 0 {
        None
    } else {
        Some(tot_merged as f64 / tot_opened as f64)
    };
    let tot_ratio = if tot_opened == 0 {
        None
    } else {
        Some(tot_reviewed as f64 / tot_opened as f64)
    };

    rsx! {
        div { class: "ts-defs-bar",
            button {
                class: "ts-defs-toggle",
                onclick: move |_| show_defs.toggle(),
                Icon { width: 13, height: 13, icon: LdInfo }
                if show_defs() { "Hide column definitions" } else { "What do these columns mean?" }
            }
        }
        if show_defs() {
            div { class: "ts-defs",
                dl {
                    dt { "Opened" }
                    dd {
                        "PRs they authored that were " b { "created" } " inside the selected window, in tracked repos, "
                        "and that are " b { "asking for review" } ". Still-draft PRs are excluded — they request "
                        "nothing, and counting them would penalise anyone who works in drafts. "
                        span { class: "muted", "Measured here, 2.8% of open drafts had a human review against 24.8% of ready ones." }
                    }

                    dt { "Drafts" }
                    dd {
                        "How many of their PRs in the window are still drafts. Context only — never counted in "
                        "Opened, so it never moves either ratio."
                    }

                    dt { "Merged" }
                    dd {
                        "How many of those same PRs have since merged — counted by when the PR was "
                        i { "opened" } ", not when it merged. A PR opened in the window and merged later still counts here."
                    }

                    dt { "Merged / Opened" }
                    dd {
                        "Share of the PRs they opened in the window that have merged " b { "so far" } ". "
                        "Because Merged counts by when a PR was opened, one opened late in the window may "
                        "simply not have had time to land yet — a low rate at the recent end is expected, "
                        "not a signal. A rate well below the team's own is worth a look for abandoned work."
                    }

                    dt { "Reviewed" }
                    dd {
                        "Distinct PRs " b { "written by someone else" }
                        " on which they submitted at least one review during the window. Counts each PR once however "
                        "many times they reviewed it. "
                        span { class: "muted", "Self-reviews and bot accounts are excluded." }
                    }

                    dt { "Approvals" }
                    dd {
                        "Times they hit " b { "Approve" } " on someone else's PR during the window. This counts approval "
                        i { "events" } " where Reviewed counts distinct PRs, so the two differ in both directions: "
                        "re-approving a PR after another round of changes adds to Approvals only, and reviewing "
                        "without approving — comments, or requesting changes — adds to Reviewed only."
                    }

                    dt { "Received" }
                    dd {
                        "Human reviews landed on the PRs they opened in the window, over those PRs' whole lifetime "
                        "(not just reviews submitted inside the window). The review capacity they consumed."
                    }

                    dt { "Queue" }
                    dd {
                        "Open, non-draft PRs where they are currently a requested reviewer and have not reviewed yet. "
                        span { class: "muted", "A live snapshot — it ignores the window, and GitHub exposes no \"requested at\" time, so it cannot say how long." }
                    }

                    dt { "Reviewed / Opened" }
                    dd {
                        "The reciprocity number. Below "
                        span { class: "ts-ratio-value bad", "1.00" }
                        " they ask for more review than they give; above it they are subsidising everyone else. "
                        span { class: "muted", "Shown as ∞ for someone who reviewed but opened nothing — that is not a score, just an undefined ratio." }
                    }

                    dt { "Trend" }
                    dd { "Distinct PRs reviewed each week across the window. Every row uses the same vertical scale, so heights compare directly." }
                }
                p { class: "hint",
                    "Everything except Queue is scoped to the window and to the repos ticked for stats."
                }
            }
        }
        div { class: "ts-table-wrap",
            table { class: "gh-table ts-table",
                thead {
                    tr {
                        th { "Member" }
                        th {
                            class: "num",
                            title: "PRs they authored in this window that are asking for review. Still-draft PRs are excluded",
                            "Opened"
                        }
                        th {
                            class: "num",
                            title: "Of their PRs in this window, how many are still drafts. Not counted in Opened",
                            "Drafts"
                        }
                        th { class: "num", title: "Of those, how many have merged (whenever they merged)", "Merged" }
                        th {
                            class: "num",
                            title: "Share of the PRs they opened that have merged so far. Recent PRs may simply not have had time yet",
                            "Merged / Opened"
                        }
                        th {
                            class: "num",
                            title: "Distinct PRs by someone else on which they submitted a review in this window. Bots and self-reviews excluded",
                            "Reviewed"
                        }
                        th {
                            class: "num",
                            title: "Times they approved someone else's PR in this window. Counts approval events, not distinct PRs",
                            "Approvals"
                        }
                        th {
                            class: "num",
                            title: "Human reviews the PRs they opened in this window have received, over those PRs' whole lifetime",
                            "Received"
                        }
                        th {
                            class: "num",
                            title: "Open non-draft PRs where they are a requested reviewer and have not reviewed yet. Live, not window-scoped",
                            "Queue"
                        }
                        th {
                            class: "num",
                            title: "Reviewed divided by Opened. Below 1.00 means they ask for more review than they give",
                            "Reviewed / Opened"
                        }
                        th { class: "num", title: "Distinct PRs reviewed per week. Same scale on every row", "Trend" }
                    }
                }
                tbody {
                    for p in rows.iter() {
                        tr { key: "{p.login}",
                            td {
                                span { class: "ts-member",
                                    span { class: "badge badge-{p.member_group}", "{p.member_group}" }
                                    span { "{p.login}" }
                                }
                            }
                            td { class: "num", "{p.prs_opened}" }
                            td { class: "num muted",
                                if p.prs_draft == 0 { "—" } else { "{p.prs_draft}" }
                            }
                            td { class: "num", "{p.prs_merged}" }
                            td { class: "num",
                                match p.merge_ratio() {
                                    Some(r) => rsx! { span { class: "mono", "{r:.2}" } },
                                    None => rsx! { span { class: "muted", "—" } },
                                }
                            }
                            td { class: "num", "{p.prs_reviewed}" }
                            td { class: "num", "{p.approvals}" }
                            td { class: "num muted", "{p.reviews_received}" }
                            td { class: "num muted", "{p.open_review_requests}" }
                            td { class: "num",
                                match p.ratio() {
                                    // No ratio to show. Distinguish "did nothing
                                    // at all" from "reviewed but opened nothing" —
                                    // in a quiet window every row would otherwise
                                    // read ∞, which looks like a score.
                                    None if p.prs_reviewed == 0 => rsx! {
                                        span { class: "muted", title: "no activity in this window", "—" }
                                    },
                                    None => rsx! {
                                        span { class: "muted", title: "reviewed, but opened nothing in this window", "∞" }
                                    },
                                    Some(r) => rsx! {
                                        div { class: "ts-ratio",
                                            span { class: "ts-ratio-value {ratio_tone(r)}", "{r:.2}" }
                                            div { class: "ts-bar",
                                                div {
                                                    class: "ts-bar-fill {ratio_tone(r)}",
                                                    style: "width:{(r.min(2.0) / 2.0 * 100.0):.0}%;",
                                                }
                                            }
                                        }
                                    },
                                }
                            }
                            td {
                                Sparkline {
                                    values: spark.get(&p.login.to_lowercase()).cloned().unwrap_or_default(),
                                    color: group_color(&p.member_group).to_string(),
                                    max: spark_max,
                                }
                            }
                        }
                    }
                }
                tfoot {
                    tr {
                        td { "All shown" }
                        td { class: "num", "{tot_opened}" }
                        td { class: "num muted",
                            if tot_draft == 0 { "—" } else { "{tot_draft}" }
                        }
                        td { class: "num", "{tot_merged}" }
                        td { class: "num",
                            match tot_merge_ratio {
                                Some(r) => rsx! { span { class: "mono", "{r:.2}" } },
                                None => rsx! { span { class: "muted", "—" } },
                            }
                        }
                        td { class: "num", "{tot_reviewed}" }
                        td { class: "num", "{tot_appr}" }
                        td { class: "num", "" }
                        td { class: "num", "" }
                        td { class: "num",
                            match tot_ratio {
                                Some(r) => rsx! { span { class: "ts-ratio-value {ratio_tone(r)}", "{r:.2}" } },
                                None => rsx! { span { class: "muted", "—" } },
                            }
                        }
                        td { "" }
                    }
                }
            }
        }
    }
}
