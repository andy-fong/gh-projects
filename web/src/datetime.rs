//! Date helpers, in pure Rust plus `js_sys::Date` for "now".
//!
//! The web crate has no `chrono`/`time` dependency on purpose — these were
//! extracted from `pages/calendar.rs` so the calendar and Team Stats share one
//! implementation and can't drift.

pub fn today_ymd() -> (i32, i32, i32) {
    let d = js_sys::Date::new_0();
    (
        d.get_full_year() as i32,
        d.get_month() as i32 + 1,
        d.get_date() as i32,
    )
}

pub fn add_months(year: i32, month: i32, delta: i32) -> (i32, i32) {
    let total = (year * 12 + month - 1) + delta;
    (total.div_euclid(12), total.rem_euclid(12) + 1)
}

pub fn days_in_month(year: i32, month: i32) -> i32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        _ => 28,
    }
}

/// Tomohiko Sakamoto: 0=Sun … 6=Sat
pub fn day_of_week(year: i32, month: i32, day: i32) -> i32 {
    let t = [0i32, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    (y + y / 4 - y / 100 + y / 400 + t[(month - 1) as usize] + day).rem_euclid(7)
}

pub fn full_month_name(month: i32) -> &'static str {
    [
        "January", "February", "March", "April", "May", "June", "July", "August", "September",
        "October", "November", "December",
    ][(month - 1) as usize]
}

pub fn short_month_name(month: i32) -> &'static str {
    [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ][(month - 1) as usize]
}

/// Parses a leading `YYYY-MM-DD`, ignoring any time suffix.
pub fn parse_date(s: &str) -> Option<(i32, i32, i32)> {
    let head = s.get(..10).unwrap_or(s);
    let mut p = head.splitn(3, '-');
    Some((
        p.next()?.parse().ok()?,
        p.next()?.parse().ok()?,
        p.next()?.parse().ok()?,
    ))
}

/// Julian Day Number — exact integer day arithmetic, no external crate needed.
pub fn jdn(y: i32, m: i32, d: i32) -> i32 {
    let a = (14 - m) / 12;
    let y2 = y + 4800 - a;
    let m2 = m + 12 * a - 3;
    d + (153 * m2 + 2) / 5 + 365 * y2 + y2 / 4 - y2 / 100 + y2 / 400 - 32045
}

/// Coarse "N days ago" label. Shared with the calendar's countdown badge so the
/// two always word elapsed time the same way.
pub fn ago_label(days: i32) -> String {
    if days < 14 {
        format!("{days}d ago")
    } else if days < 90 {
        format!("{}w ago", days / 7)
    } else {
        format!("~{}mo ago", days / 30)
    }
}

/// Fine-grained "2h ago" for the sync timestamp, which needs sub-day
/// resolution. Falls back to [`ago_label`] past a day.
///
/// Accepts both RFC3339 (`2026-09-13T21:00:00Z`) and SQLite's
/// `datetime('now')` form (`2026-09-13 21:00:00`), which has no zone and is
/// UTC — `js_sys::Date` would otherwise read it as local time.
pub fn relative_from_now(iso: &str) -> String {
    let normalized = if iso.contains(' ') && !iso.ends_with('Z') {
        format!("{}Z", iso.replace(' ', "T"))
    } else {
        iso.to_string()
    };
    let t = js_sys::Date::new(&wasm_bindgen::JsValue::from_str(&normalized)).get_time();
    if t.is_nan() {
        return "unknown".to_string();
    }
    let mins = ((js_sys::Date::now() - t) / 60000.0).floor().max(0.0) as i64;
    match mins {
        0 => "just now".to_string(),
        1..=59 => format!("{mins}m ago"),
        60..=1439 => format!("{}h ago", mins / 60),
        _ => ago_label((mins / 1440) as i32),
    }
}

/// `"2026-09-07"` → `"Sep 7"`, for chart axis labels.
pub fn week_label(ymd: &str) -> String {
    match parse_date(ymd) {
        Some((_, m, d)) => format!("{} {}", short_month_name(m), d),
        None => ymd.to_string(),
    }
}
