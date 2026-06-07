use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{LdCopy, LdX};
use dioxus_free_icons::Icon;

use crate::platform;

/// Shows a generated Slack status update for the war room, with one-click copy.
/// The textarea is editable so the user can tweak before pasting. The "Include
/// item details" toggle switches between the full update (`detailed`) and a
/// product/version-only headline (`summary`).
#[component]
pub fn SlackUpdateDialog(
    detailed: String,
    summary: String,
    on_close: EventHandler<()>,
) -> Element {
    let mut include_details = use_signal(|| true);
    let mut body = use_signal(|| detailed.clone());
    let mut copied = use_signal(|| false);

    rsx! {
        div { class: "modal-backdrop", onclick: move |_| on_close.call(()),
            div {
                class: "modal",
                style: "width: 640px;",
                onclick: move |e| e.stop_propagation(),
                div { class: "modal-header",
                    h2 { class: "modal-title", "Slack update" }
                    button { class: "icon-btn", onclick: move |_| on_close.call(()),
                        Icon { width: 16, height: 16, icon: LdX }
                    }
                }
                div { class: "hint", style: "margin-bottom:8px;",
                    "Generated from the current item statuses. Edit if needed, then copy."
                }
                label { class: "wr-step", style: "margin-bottom:8px;",
                    input {
                        r#type: "checkbox",
                        checked: include_details(),
                        onchange: {
                            let detailed = detailed.clone();
                            let summary = summary.clone();
                            move |e: Event<FormData>| {
                                let on = e.checked();
                                include_details.set(on);
                                body.set(if on { detailed.clone() } else { summary.clone() });
                                copied.set(false);
                            }
                        },
                    }
                    "Include item details"
                }
                textarea {
                    class: "textarea mono",
                    rows: 18,
                    style: "width:100%;",
                    value: "{body}",
                    oninput: move |e| {
                        body.set(e.value());
                        copied.set(false);
                    },
                }
                div { class: "form-actions",
                    button {
                        class: "btn btn-ghost",
                        r#type: "button",
                        onclick: move |_| on_close.call(()),
                        "Close"
                    }
                    button {
                        class: "btn btn-primary",
                        r#type: "button",
                        onclick: move |_| {
                            platform::copy_to_clipboard(&body.read());
                            copied.set(true);
                        },
                        Icon { width: 16, height: 16, icon: LdCopy }
                        if copied() {
                            "Copied!"
                        } else {
                            "Copy"
                        }
                    }
                }
            }
        }
    }
}
