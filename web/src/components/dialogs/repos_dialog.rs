use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{LdPlus, LdTrash2, LdX};
use dioxus_free_icons::Icon;

use crate::api;
use crate::state::use_app_state;
use crate::types::{CreateRepoInput, UpdateRepoInput};

/// Manage the global repo registry (friendly name → owner/repo). War room
/// groups pick from these.
#[component]
pub fn ReposDialog(on_close: EventHandler<()>) -> Element {
    let state = use_app_state();

    let repos = use_resource(move || {
        let _ = state.repos_ver.read();
        async move { api::repos::list().await }
    });

    let mut new_name = use_signal(String::new);
    let mut new_owner_repo = use_signal(String::new);

    let add = move || {
        let name = new_name.read().trim().to_string();
        let owner_repo = new_owner_repo.read().trim().to_string();
        if name.is_empty() || owner_repo.is_empty() {
            return;
        }
        spawn(async move {
            if api::repos::create(&CreateRepoInput { name, owner_repo }).await.is_ok() {
                new_name.set(String::new());
                new_owner_repo.set(String::new());
                state.invalidate_repos();
            }
        });
    };

    let items = match repos.read().as_ref() {
        Some(Ok(v)) => v.clone(),
        _ => Vec::new(),
    };

    rsx! {
        div { class: "modal-backdrop", onclick: move |_| on_close.call(()),
            div {
                class: "modal",
                style: "width: 560px;",
                onclick: move |e| e.stop_propagation(),
                div { class: "modal-header",
                    h2 { class: "modal-title", "Repos" }
                    button { class: "icon-btn", onclick: move |_| on_close.call(()),
                        Icon { width: 16, height: 16, icon: LdX }
                    }
                }
                div { class: "hint", style: "margin-bottom:12px;",
                    "Reusable across all war rooms. Each entry is a friendly name paired with an owner/repo."
                }

                for r in items {
                    RepoRow { key: "{r.id}", id: r.id, name: r.name.clone(), owner_repo: r.owner_repo.clone() }
                }

                div { class: "wr-repo-row",
                    input {
                        class: "input",
                        placeholder: "Friendly name (Kgateway OSS)",
                        value: "{new_name}",
                        oninput: move |e| new_name.set(e.value()),
                    }
                    input {
                        class: "input",
                        placeholder: "owner/repo",
                        value: "{new_owner_repo}",
                        oninput: move |e| new_owner_repo.set(e.value()),
                        onkeydown: {
                            let a = add;
                            move |e: Event<KeyboardData>| if e.key() == Key::Enter { a() }
                        },
                    }
                    button {
                        class: "btn btn-primary btn-sm",
                        onclick: {
                            let a = add;
                            move |_| a()
                        },
                        Icon { width: 14, height: 14, icon: LdPlus }
                        "Add"
                    }
                }
            }
        }
    }
}

#[component]
fn RepoRow(id: i64, name: String, owner_repo: String) -> Element {
    let state = use_app_state();
    let mut name_sig = use_signal(|| name.clone());
    let mut repo_sig = use_signal(|| owner_repo.clone());

    let save = move |_| {
        let name = name_sig.read().trim().to_string();
        let owner_repo = repo_sig.read().trim().to_string();
        if name.is_empty() || owner_repo.is_empty() {
            return;
        }
        spawn(async move {
            let _ = api::repos::update(
                id,
                &UpdateRepoInput {
                    name: Some(name),
                    owner_repo: Some(owner_repo),
                    position: None,
                },
            )
            .await;
            state.invalidate_repos();
        });
    };

    let remove = move |_| {
        spawn(async move {
            if api::repos::delete(id).await.is_ok() {
                state.invalidate_repos();
            }
        });
    };

    rsx! {
        div { class: "wr-repo-row",
            input {
                class: "input",
                value: "{name_sig}",
                oninput: move |e| name_sig.set(e.value()),
                onblur: save,
            }
            input {
                class: "input",
                value: "{repo_sig}",
                oninput: move |e| repo_sig.set(e.value()),
                onblur: save,
            }
            button { class: "icon-btn danger", title: "Delete", onclick: remove,
                Icon { width: 16, height: 16, icon: LdTrash2 }
            }
        }
    }
}
