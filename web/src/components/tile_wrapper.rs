use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{LdCopy, LdSettings, LdTrash2};
use dioxus_free_icons::Icon;

/// Tile chrome: title bar with hover actions (copy/edit/delete) + body.
/// The drag handle is dropped in the simplified layout.
#[component]
pub fn TileWrapper(
    title: String,
    on_delete: EventHandler<()>,
    on_edit: EventHandler<()>,
    on_copy: EventHandler<()>,
    children: Element,
) -> Element {
    let mut hovered = use_signal(|| false);

    rsx! {
        div {
            class: "tile",
            onmouseenter: move |_| hovered.set(true),
            onmouseleave: move |_| hovered.set(false),
            div { class: "tile-head",
                div { class: "tile-head-title",
                    span { class: "tile-title", "{title}" }
                }
                if hovered() {
                    div { class: "tile-actions",
                        button {
                            class: "icon-btn",
                            title: "Copy tile",
                            onclick: move |_| on_copy.call(()),
                            Icon { width: 14, height: 14, icon: LdCopy }
                        }
                        button {
                            class: "icon-btn",
                            title: "Edit tile",
                            onclick: move |_| on_edit.call(()),
                            Icon { width: 14, height: 14, icon: LdSettings }
                        }
                        button {
                            class: "icon-btn danger",
                            title: "Delete tile",
                            onclick: move |_| on_delete.call(()),
                            Icon { width: 14, height: 14, icon: LdTrash2 }
                        }
                    }
                }
            }
            div { class: "tile-body", {children} }
        }
    }
}
