use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::LdX;
use dioxus_free_icons::Icon;

/// Pick an item from another group to mirror (read-only) into this group.
/// `candidates` are `(source_item_id, "Group / Title")` for eligible items.
#[component]
pub fn WarRoomLinkDialog(
    candidates: Vec<(i64, String)>,
    on_confirm: EventHandler<i64>,
    on_close: EventHandler<()>,
) -> Element {
    let mut selected = use_signal(|| {
        candidates.first().map(|(id, _)| id.to_string()).unwrap_or_default()
    });

    let none_available = candidates.is_empty();

    let submit = move |_| {
        if let Ok(id) = selected.read().parse::<i64>() {
            on_confirm.call(id);
        }
    };

    rsx! {
        div { class: "modal-backdrop", onclick: move |_| on_close.call(()),
            div {
                class: "modal",
                style: "width: 460px;",
                onclick: move |e| e.stop_propagation(),
                div { class: "modal-header",
                    h2 { class: "modal-title", "Link an item" }
                    button { class: "icon-btn", onclick: move |_| on_close.call(()),
                        Icon { width: 16, height: 16, icon: LdX }
                    }
                }
                if none_available {
                    div { class: "hint", "No items in other groups are available to link." }
                    div { class: "form-actions",
                        button {
                            class: "btn btn-ghost",
                            r#type: "button",
                            onclick: move |_| on_close.call(()),
                            "Close"
                        }
                    }
                } else {
                    form { onsubmit: submit,
                        div { class: "field",
                            label { "Source item" }
                            select {
                                class: "select",
                                value: "{selected}",
                                onchange: move |e| selected.set(e.value()),
                                for (id , label) in candidates.iter() {
                                    option { value: "{id}", "{label}" }
                                }
                            }
                            div { class: "hint",
                                "A read-only mirror is added here; edit the original to update it everywhere."
                            }
                        }
                        div { class: "form-actions",
                            button {
                                class: "btn btn-ghost",
                                r#type: "button",
                                onclick: move |_| on_close.call(()),
                                "Cancel"
                            }
                            button { class: "btn btn-primary", r#type: "submit", "Link item" }
                        }
                    }
                }
            }
        }
    }
}
