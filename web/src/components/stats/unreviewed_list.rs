//! The actionable end of the page: open PRs that need a reviewer.

use dioxus::prelude::*;

use crate::types::WorklistRow;

fn age_class(days: i64) -> &'static str {
    if days > 30 {
        "ts-age ts-age-bad"
    } else if days > 7 {
        "ts-age ts-age-warn"
    } else {
        "ts-age ts-age-fresh"
    }
}

#[component]
pub fn UnreviewedList(rows: Vec<WorklistRow>, on_open: EventHandler<WorklistRow>) -> Element {
    let mut expanded = use_signal(|| false);

    if rows.is_empty() {
        return rsx! { div { class: "empty-hint", "Nothing is waiting for a reviewer. " } };
    }
    let cap = if expanded() { rows.len() } else { rows.len().min(12) };

    rsx! {
        div { class: "ts-work",
            for r in rows.iter().take(cap) {
                {
                    let row = r.clone();
                    rsx! {
                        div {
                            key: "{r.repo}#{r.number}",
                            class: "ts-work-row",
                            onclick: move |_| on_open.call(row.clone()),
                            span { class: age_class(r.age_days), "{r.age_days}d" }
                            span { class: "ts-work-ref mono",
                                "{r.repo.split('/').last().unwrap_or(&r.repo)}#{r.number}"
                            }
                            span { class: "ts-work-title", "{r.title}" }
                            span { class: "badge badge-{r.author_group}",
                                "{r.author_login.clone().unwrap_or_else(|| \"unknown\".into())}"
                            }
                            // A PR an AI reviewed and no human did is the exact
                            // failure this page exists to surface.
                            if r.bot_review_events > 0 {
                                span { class: "ts-work-ai", title: "Only an AI has reviewed this", "AI only" }
                            }
                            span { class: "ts-work-wait muted text-xs",
                                match r.requested_reviewers.as_deref() {
                                    Some(w) if !w.is_empty() => rsx! { "→ {w}" },
                                    // No requested reviewer at all: nobody owns it.
                                    _ => rsx! { span { class: "ts-work-nowait", "→ nobody assigned" } },
                                }
                            }
                        }
                    }
                }
            }
            if rows.len() > 12 {
                button {
                    class: "btn btn-sm ts-work-more",
                    onclick: move |_| expanded.toggle(),
                    if expanded() { "Show fewer" } else { "Show all {rows.len()}" }
                }
            }
        }
    }
}
