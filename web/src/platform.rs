//! Thin wrappers over browser APIs: localStorage (typed list values) and file
//! download. Used for per-tile UI prefs and backup export.

use gloo_storage::{LocalStorage, Storage};
use wasm_bindgen::JsCast;

/// Read a JSON-array localStorage value (e.g. `["url"]`). Compatible with the
/// old React app's `JSON.stringify(array)` format.
pub fn get_list(key: &str) -> Option<Vec<String>> {
    LocalStorage::get::<Vec<String>>(key).ok()
}

pub fn set_list(key: &str, val: &Vec<String>) {
    let _ = LocalStorage::set(key, val);
}

pub fn remove_key(key: &str) {
    LocalStorage::delete(key);
}

/// All localStorage keys starting with any of the given prefixes.
pub fn keys_with_prefix(prefixes: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let len = storage.length().unwrap_or(0);
        for i in 0..len {
            if let Ok(Some(k)) = storage.key(i) {
                if prefixes.iter().any(|p| k.starts_with(p)) {
                    out.push(k);
                }
            }
        }
    }
    out
}

/// Trigger a browser download of `content` as `filename`.
pub fn download_text(filename: &str, content: &str) {
    let parts = js_sys::Array::new();
    parts.push(&wasm_bindgen::JsValue::from_str(content));
    let blob = match web_sys::Blob::new_with_str_sequence(&parts) {
        Ok(b) => b,
        Err(_) => return,
    };
    let url = match web_sys::Url::create_object_url_with_blob(&blob) {
        Ok(u) => u,
        Err(_) => return,
    };
    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
        if let Ok(el) = document.create_element("a") {
            if let Ok(anchor) = el.dyn_into::<web_sys::HtmlAnchorElement>() {
                anchor.set_href(&url);
                anchor.set_download(filename);
                anchor.click();
            }
        }
    }
    let _ = web_sys::Url::revoke_object_url(&url);
}
