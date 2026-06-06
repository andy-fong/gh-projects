use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{
    LdCircleDot, LdExternalLink, LdGitMerge, LdGitPullRequest, LdMessageSquare, LdPlus,
    LdStickyNote, LdX,
};
use dioxus_free_icons::Icon;
use serde_json::Value;

use crate::api;
use crate::components::common::{Avatar, ProseStripped};
use crate::components::dialogs::note_dialog::NoteDialog;
use crate::field_extractors::format_date_short;
use crate::state::use_app_state;
use crate::types::CreateNoteInput;

fn s(v: &Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

#[derive(Clone, PartialEq)]
struct Person {
    login: String,
    name: String,
}

#[derive(Clone, PartialEq)]
struct Comment {
    author: String,
    body: String,
    created_at: String,
}

fn person(v: &Value) -> Person {
    Person {
        login: s(v, "login"),
        name: {
            let n = s(v, "name");
            if n.is_empty() { s(v, "login") } else { n }
        },
    }
}

#[component]
pub fn DetailPanel(
    repo: String,
    ref_type: String,
    number: i64,
    url: String,
    on_close: EventHandler<()>,
) -> Element {
    let state = use_app_state();
    let mut show_note_dialog = use_signal(|| false);

    let is_pr = ref_type == "pr";
    let json_fields = if is_pr {
        "number,title,state,body,author,assignees,labels,milestone,comments,url,createdAt,updatedAt,isDraft,headRefName,baseRefName,reviewDecision"
    } else {
        "number,title,state,body,author,assignees,labels,milestone,comments,url,createdAt,updatedAt"
    };
    let cmd = format!("{ref_type} view {number} --repo {repo} --json {json_fields}");

    let gh = use_resource({
        let cmd = cmd.clone();
        move || {
            let cmd = cmd.clone();
            async move { api::gh::execute(&cmd).await }
        }
    });

    let notes = use_resource({
        let repo = repo.clone();
        let ref_type = ref_type.clone();
        move || {
            let _ = state.notes_ver.read();
            let repo = repo.clone();
            let ref_type = ref_type.clone();
            async move {
                api::notes::list(api::notes::Filter {
                    repo: Some(repo),
                    ref_type: Some(ref_type),
                    ref_number: Some(number),
                })
                .await
            }
        }
    });

    let gh_read = gh.read();
    let item: Option<Value> = match gh_read.as_ref() {
        Some(Ok(resp)) => Some(resp.output.clone()),
        _ => None,
    };
    let gh_error: Option<String> = match gh_read.as_ref() {
        Some(Err(e)) => Some(e.clone()),
        _ => None,
    };
    let loading = gh_read.is_none();

    let note_list = match notes.read().as_ref() {
        Some(Ok(v)) => v.clone(),
        _ => Vec::new(),
    };

    // For the note dialog prefill / create handler.
    let dialog_initial = CreateNoteInput {
        repo: Some(repo.clone()),
        ref_type: Some(ref_type.clone()),
        ref_number: Some(number),
        ..Default::default()
    };
    let on_create = {
        move |input: CreateNoteInput| {
            spawn(async move {
                if api::notes::create(&input).await.is_ok() {
                    state.invalidate_notes();
                }
            });
            show_note_dialog.set(false);
        }
    };

    rsx! {
        div { class: "panel-backdrop", onclick: move |_| on_close.call(()) }
        div { class: "detail-panel",
            div { class: "detail-head",
                div { style: "flex:1; min-width:0;",
                    if loading {
                        div { class: "loading-text", "Loading…" }
                    }
                    if let Some(err) = gh_error {
                        div { class: "error-text", "{err}" }
                    }
                    if let Some(item) = item.clone() {
                        {render_header(&item, is_pr)}
                    }
                }
                div { class: "flex gap-2", style: "flex-shrink:0;",
                    a {
                        class: "icon-btn",
                        href: "{url}",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        title: "Open on GitHub",
                        Icon { width: 16, height: 16, icon: LdExternalLink }
                    }
                    button { class: "icon-btn", onclick: move |_| on_close.call(()),
                        Icon { width: 16, height: 16, icon: LdX }
                    }
                }
            }

            div { class: "detail-body",
                if let Some(item) = item.clone() {
                    {render_body(&item, &url, &note_list, move || show_note_dialog.set(true))}
                }
            }
        }

        if show_note_dialog() {
            NoteDialog {
                initial: dialog_initial,
                on_confirm: on_create,
                on_close: move |_| show_note_dialog.set(false),
            }
        }
    }
}

fn render_header(item: &Value, is_pr: bool) -> Element {
    let number = item.get("number").and_then(|n| n.as_i64()).unwrap_or(0);
    let title = s(item, "title");
    let raw_state = s(item, "state").to_lowercase();
    let is_draft = is_pr && item.get("isDraft").and_then(|d| d.as_bool()).unwrap_or(false);
    let display_state = if is_draft { "draft".to_string() } else { raw_state.clone() };

    let badge_class = match display_state.as_str() {
        "open" => "badge badge-open",
        "closed" => {
            if is_pr {
                "badge badge-closed-pr"
            } else {
                "badge badge-closed"
            }
        }
        "merged" => "badge badge-merged",
        "draft" => "badge badge-draft",
        _ => "badge badge-open",
    };

    let author = item.get("author").map(person).unwrap_or(Person {
        login: String::new(),
        name: String::new(),
    });
    let assignees: Vec<Person> = item
        .get("assignees")
        .and_then(|a| a.as_array())
        .map(|arr| arr.iter().map(person).collect())
        .unwrap_or_default();
    let labels: Vec<String> = item
        .get("labels")
        .and_then(|a| a.as_array())
        .map(|arr| arr.iter().map(|l| s(l, "name")).collect())
        .unwrap_or_default();
    let milestone = item
        .get("milestone")
        .and_then(|m| m.get("title"))
        .and_then(|t| t.as_str())
        .map(String::from);
    let head_ref = s(item, "headRefName");
    let base_ref = s(item, "baseRefName");

    rsx! {
        div { class: "flex gap-2 items-center", style: "margin-bottom:4px;",
            span { class: "{badge_class}",
                {state_icon(&display_state, is_pr)}
                "{display_state}"
            }
            span { class: "text-xs muted", "#{number}" }
            if let Some(m) = milestone {
                span { class: "label-chip", "{m}" }
            }
        }
        h2 { style: "font-size:14px; font-weight:600; margin:0; line-height:1.3;", "{title}" }
        div { class: "flex gap-2 items-center", style: "margin-top:6px; flex-wrap:wrap;",
            span { class: "flex gap-2 items-center text-xs muted",
                Avatar { login: author.login.clone() }
                "{author.name}"
            }
            if !assignees.is_empty() {
                span { class: "flex gap-2 items-center text-xs muted",
                    "→ "
                    for a in assignees {
                        span { key: "{a.login}", class: "flex gap-2 items-center",
                            Avatar { login: a.login.clone() }
                            "{a.name}"
                        }
                    }
                }
            }
        }
        if !labels.is_empty() {
            div { class: "tag-row",
                for l in labels {
                    span { key: "{l}", class: "label-chip", "{l}" }
                }
            }
        }
        if is_pr && !head_ref.is_empty() {
            div { class: "text-xs muted", style: "margin-top:4px;",
                code { "{head_ref}" }
                " → "
                code { "{base_ref}" }
            }
        }
    }
}

fn state_icon(state: &str, is_pr: bool) -> Element {
    match state {
        "closed" => rsx! { Icon { width: 12, height: 12, icon: LdX } },
        "merged" => rsx! { Icon { width: 12, height: 12, icon: LdGitMerge } },
        "draft" => rsx! { Icon { width: 12, height: 12, icon: LdGitPullRequest } },
        _ => {
            if is_pr {
                rsx! { Icon { width: 12, height: 12, icon: LdGitPullRequest } }
            } else {
                rsx! { Icon { width: 12, height: 12, icon: LdCircleDot } }
            }
        }
    }
}

fn render_body(
    item: &Value,
    url: &str,
    notes: &[crate::types::Note],
    on_add_note: impl FnMut() + 'static,
) -> Element {
    let author = item.get("author").map(person).unwrap_or(Person {
        login: String::new(),
        name: String::new(),
    });
    let body = s(item, "body");
    let created_at = s(item, "createdAt");

    let comments: Vec<Comment> = item
        .get("comments")
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .map(|c| Comment {
                    author: c.get("author").map(|a| s(a, "login")).unwrap_or_default(),
                    body: s(c, "body"),
                    created_at: s(c, "createdAt"),
                })
                .collect()
        })
        .unwrap_or_default();
    let comment_count = comments.len();
    let last_comment = comments.last().cloned();

    let mut on_add_note = on_add_note;

    rsx! {
        section {
            CommentCard {
                author: author.login.clone(),
                body,
                created_at,
                label: "description".to_string(),
            }
        }
        if let Some(c) = last_comment {
            section {
                div { class: "section-label",
                    Icon { width: 14, height: 14, icon: LdMessageSquare }
                    "Last comment"
                    if comment_count > 1 {
                        a {
                            class: "spacer",
                            style: "text-align:right;",
                            href: "{url}",
                            target: "_blank",
                            rel: "noopener noreferrer",
                            "{comment_count} total ↗"
                        }
                    }
                }
                CommentCard {
                    author: c.author.clone(),
                    body: c.body.clone(),
                    created_at: c.created_at.clone(),
                    label: String::new(),
                }
            }
        }
        section {
            div { class: "section-label",
                Icon { width: 14, height: 14, icon: LdStickyNote }
                "Private notes ({notes.len()})"
                button {
                    class: "chip spacer",
                    style: "text-align:right; color: var(--accent);",
                    onclick: move |_| on_add_note(),
                    Icon { width: 12, height: 12, icon: LdPlus }
                    "Add note"
                }
            }
            if notes.is_empty() {
                div { class: "text-xs muted", style: "font-style:italic;", "No notes yet" }
            } else {
                div { style: "display:flex; flex-direction:column; gap:8px;",
                    for note in notes {
                        div {
                            key: "{note.id}",
                            style: "border:1px solid var(--border); border-radius:8px; padding:8px 12px;",
                            div { class: "text-xs", style: "font-weight:500; margin-bottom:4px;", "{note.title}" }
                            ProseStripped { text: note.body.clone() }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn CommentCard(author: String, body: String, created_at: String, label: String) -> Element {
    let date = if created_at.is_empty() {
        String::new()
    } else {
        format_date_short(&created_at)
    };
    rsx! {
        div { class: "comment",
            div { class: "comment-head",
                Avatar { login: author.clone() }
                span { style: "font-weight:500;", "{author}" }
                span { class: "spacer muted", style: "text-align:right;", "{date}" }
                if !label.is_empty() {
                    span { class: "muted", style: "font-style:italic;", "{label}" }
                }
            }
            div { class: "comment-body",
                ProseStripped { text: body }
            }
        }
    }
}
