use dioxus::prelude::*;

/// Renders Markdown as HTML. Replaces `react-markdown`. Empty input shows
/// "*No content*" like the old app.
#[component]
pub fn Prose(text: String, #[props(default)] small: bool) -> Element {
    let body = if text.trim().is_empty() {
        "*No content*".to_string()
    } else {
        text
    };
    let html = crate::markdown::to_html(&body);
    let class = if small { "prose small" } else { "prose" };
    rsx! {
        div { class: "{class}", dangerous_inner_html: "{html}" }
    }
}

/// Same as [`Prose`] but strips HTML comments first (GitHub issue/PR bodies).
#[component]
pub fn ProseStripped(text: String) -> Element {
    let body = if text.trim().is_empty() {
        "*No content*".to_string()
    } else {
        text
    };
    let html = crate::markdown::to_html_stripped_comments(&body);
    rsx! {
        div { class: "prose small", dangerous_inner_html: "{html}" }
    }
}

#[component]
pub fn Avatar(login: String) -> Element {
    rsx! {
        img {
            class: "avatar",
            src: "https://github.com/{login}.png?size=20",
            alt: "{login}",
        }
    }
}

/// Hover tooltip positioned near the cursor. Replaces the React portal tooltip.
/// Empty text renders the children with no wrapper behaviour.
#[component]
pub fn Tooltip(text: String, children: Element) -> Element {
    let mut pos = use_signal(|| None::<(f64, f64)>);

    if text.is_empty() {
        return rsx! {
            {children}
        };
    }

    rsx! {
        span {
            style: "display:block; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;",
            onmouseenter: move |e| {
                let c = e.client_coordinates();
                pos.set(Some((c.x, c.y)));
            },
            onmouseleave: move |_| pos.set(None),
            {children}
            if let Some((x , y)) = pos() {
                div {
                    class: "tooltip-box",
                    style: "left: {x}px; top: {y}px; transform: translateY(-110%);",
                    "{text}"
                }
            }
        }
    }
}
