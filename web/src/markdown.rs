//! Markdown → HTML rendering, replacing `react-markdown` + `remark-gfm`.
//! Rendered HTML is injected via `dangerous_inner_html`. Input is private,
//! user-authored note/GitHub content shown only to the local user.

use pulldown_cmark::{html, Options, Parser};

pub fn to_html(src: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_GFM);

    let parser = Parser::new_ext(src, options);
    let mut out = String::new();
    html::push_html(&mut out, parser);
    out
}

/// Like [`to_html`] but forces every link to open in a new tab. pulldown-cmark
/// emits anchors as `<a href="…">`, so we inject the target/rel attributes.
pub fn to_html_blank_links(src: &str) -> String {
    to_html(src).replace("<a href=", "<a target=\"_blank\" rel=\"noopener noreferrer\" href=")
}

/// Strip HTML comments before rendering (the detail panel does this for GitHub
/// issue/PR bodies which often contain `<!-- ... -->` template comments).
pub fn to_html_stripped_comments(src: &str) -> String {
    let mut cleaned = String::with_capacity(src.len());
    let mut rest = src;
    while let Some(start) = rest.find("<!--") {
        cleaned.push_str(&rest[..start]);
        if let Some(end) = rest[start..].find("-->") {
            rest = &rest[start + end + 3..];
        } else {
            rest = "";
            break;
        }
    }
    cleaned.push_str(rest);
    to_html(&cleaned)
}
