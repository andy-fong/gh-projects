//! The five-number "how bad is it" strip.

use dioxus::prelude::*;

use crate::types::StatsTotals;

fn hours_label(h: Option<f64>) -> String {
    match h {
        None => "—".into(),
        Some(h) if h < 48.0 => format!("{h:.1} h"),
        Some(h) => format!("{:.1} d", h / 24.0),
    }
}

#[component]
fn KpiCard(label: String, value: String, sub: String, tone: String) -> Element {
    rsx! {
        div { class: "ts-kpi",
            div { class: "ts-kpi-label", "{label}" }
            div { class: "ts-kpi-value {tone}", "{value}" }
            div { class: "ts-kpi-sub", "{sub}" }
        }
    }
}

#[component]
pub fn KpiStrip(totals: StatsTotals) -> Element {
    let t = &totals;

    // p90 is the honest one: a median over only the PRs that got reviewed looks
    // healthy precisely when the tail is worst.
    let p90_tone = match t.p90_ttfr_hours {
        Some(h) if h > 168.0 => "bad",
        Some(h) if h > 48.0 => "warn",
        _ => "",
    };
    let stale_tone = if t.open_unreviewed_7d > 20 {
        "bad"
    } else if t.open_unreviewed_7d > 5 {
        "warn"
    } else {
        ""
    };
    let conc_tone = match t.top2_review_share {
        Some(p) if p >= 60.0 => "bad",
        Some(p) if p >= 40.0 => "warn",
        _ => "",
    };

    rsx! {
        div { class: "ts-kpis",
            KpiCard {
                label: "Unreviewed > 7d".to_string(),
                value: t.open_unreviewed_7d.to_string(),
                sub: format!("of {} open with no human review", t.open_unreviewed),
                tone: stale_tone.to_string(),
            }
            KpiCard {
                label: "p90 to 1st review".to_string(),
                value: hours_label(t.p90_ttfr_hours),
                sub: format!("median {}", hours_label(t.median_ttfr_hours)),
                tone: p90_tone.to_string(),
            }
            // A PR the bot closed is work that timed out rather than being
            // decided on — the end state of the review backlog, so it belongs
            // next to it.
            KpiCard {
                label: "Closed by stale bot".to_string(),
                value: t.stale_closed.to_string(),
                sub: format!(
                    "{} marked stale · {} stale right now",
                    t.stale_marked, t.stale_open_now
                ),
                tone: if t.stale_closed > 0 { "bad".to_string() }
                      else if t.stale_open_now > 0 { "warn".to_string() }
                      else { String::new() },
            }
            KpiCard {
                label: "Review concentration".to_string(),
                value: match t.top2_review_share {
                    Some(p) => format!("{p:.0}%"),
                    None => "—".into(),
                },
                sub: format!("top 2 of {} active reviewers", t.active_reviewers),
                tone: conc_tone.to_string(),
            }
            KpiCard {
                label: "AI reviews".to_string(),
                value: t.bot_review_events.to_string(),
                sub: format!("vs {} human · {} self", t.review_events, t.self_review_events),
                tone: if t.bot_review_events > t.review_events { "warn".to_string() } else { String::new() },
            }
        }
    }
}
