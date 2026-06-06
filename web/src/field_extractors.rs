//! Port of `frontend/src/lib/fieldExtractors.ts`.
//!
//! Maps a column name to the sub-field(s) to display when the value is an
//! object. Use "|" for fallbacks: "name|login" tries "name" first, falls back
//! to "login".

use serde_json::Value;
use std::collections::BTreeMap;

pub fn default_field_extractors() -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    m.insert("author".to_string(), "login".to_string());
    m.insert("repository".to_string(), "nameWithOwner".to_string());
    m.insert("assignees".to_string(), "name|login".to_string());
    m
}

fn pick_field(obj: &serde_json::Map<String, Value>, extractor: &str) -> String {
    for field in extractor.split('|') {
        if let Some(val) = obj.get(field.trim()) {
            let s = value_to_plain(val);
            if !s.is_empty() {
                return s;
            }
        }
    }
    String::new()
}

/// A scalar JSON value as a plain string (no quotes for strings); objects/arrays
/// fall back to compact JSON.
fn value_to_plain(val: &Value) -> String {
    match val {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

pub fn extract_display(val: &Value, extractor: Option<&str>) -> String {
    match val {
        Value::Null => String::new(),
        Value::Array(arr) => {
            if arr.is_empty() {
                return String::new();
            }
            arr.iter()
                .map(|item| match (extractor, item) {
                    (Some(ex), Value::Object(o)) => pick_field(o, ex),
                    (_, Value::Object(_)) | (_, Value::Array(_)) => item.to_string(),
                    _ => value_to_plain(item),
                })
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(", ")
        }
        Value::Object(o) => {
            if let Some(ex) = extractor {
                let result = pick_field(o, ex);
                if !result.is_empty() {
                    return result;
                }
            }
            val.to_string()
        }
        _ => value_to_plain(val),
    }
}

/// Matches `^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}`.
pub fn is_datetime_string(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() < 16 {
        return false;
    }
    let d = |i: usize| b[i].is_ascii_digit();
    d(0) && d(1) && d(2) && d(3)
        && b[4] == b'-'
        && d(5) && d(6)
        && b[7] == b'-'
        && d(8) && d(9)
        && b[10] == b'T'
        && d(11) && d(12)
        && b[13] == b':'
        && d(14) && d(15)
}

/// `new Date(s).toLocaleDateString(undefined, { year, month: 'short', day })`.
/// Uses the browser's JS `Date` so locale formatting matches the old app.
pub fn format_date_only(s: &str) -> String {
    let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_str(s));
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &opts,
        &"year".into(),
        &"numeric".into(),
    );
    let _ = js_sys::Reflect::set(&opts, &"month".into(), &"short".into());
    let _ = js_sys::Reflect::set(&opts, &"day".into(), &"numeric".into());
    let undefined = wasm_bindgen::JsValue::undefined();
    date.to_locale_date_string("en-US", &opts)
        .as_string()
        .unwrap_or_else(|| {
            let _ = &undefined;
            s.to_string()
        })
}

/// `new Date(s).toLocaleDateString()` — short locale date for note timestamps.
pub fn format_date_short(s: &str) -> String {
    let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_str(s));
    let opts = js_sys::Object::new();
    date.to_locale_date_string("en-US", &opts)
        .as_string()
        .unwrap_or_else(|| s.to_string())
}
