use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{
    LdCalendar, LdChevronLeft, LdChevronRight, LdCopy, LdPencil, LdPlus, LdTrash2, LdX,
};
use dioxus_free_icons::Icon;

use crate::api;
use crate::state::use_app_state;
use crate::types::*;

// ── Date helpers ──────────────────────────────────────────────────────────────

fn today_ymd() -> (i32, i32, i32) {
    let d = js_sys::Date::new_0();
    (
        d.get_full_year() as i32,
        d.get_month() as i32 + 1,
        d.get_date() as i32,
    )
}

fn add_months(year: i32, month: i32, delta: i32) -> (i32, i32) {
    let total = (year * 12 + month - 1) + delta;
    (total.div_euclid(12), total.rem_euclid(12) + 1)
}

fn days_in_month(year: i32, month: i32) -> i32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        _ => 28,
    }
}

// Tomohiko Sakamoto: 0=Sun … 6=Sat
fn day_of_week(year: i32, month: i32, day: i32) -> i32 {
    let t = [0i32, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    (y + y / 4 - y / 100 + y / 400 + t[(month - 1) as usize] + day).rem_euclid(7)
}

fn full_month_name(month: i32) -> &'static str {
    [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ][(month - 1) as usize]
}

fn short_month_name(month: i32) -> &'static str {
    [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ][(month - 1) as usize]
}

fn parse_date(s: &str) -> Option<(i32, i32, i32)> {
    let mut p = s.splitn(3, '-');
    Some((
        p.next()?.parse().ok()?,
        p.next()?.parse().ok()?,
        p.next()?.parse().ok()?,
    ))
}

// Display name for calendar pills: short_name if set, else full name.
fn comp_display_name(c: &CalendarComponent) -> &str {
    c.short_name.as_deref().unwrap_or(&c.name)
}

fn md_to_html(text: &str) -> String {
    use pulldown_cmark::{html, Options, Parser};
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(text, opts);
    let mut out = String::new();
    html::push_html(&mut out, parser);
    out.replace(
        "<a href=",
        "<a target=\"_blank\" rel=\"noopener noreferrer\" href=",
    )
}

fn status_label(s: &str) -> &'static str {
    match s {
        "at_risk" => "At risk",
        "delayed" => "Delayed",
        "released" => "Released",
        _ => "On track",
    }
}

fn status_css(s: &str) -> &'static str {
    match s {
        "at_risk" => "status-badge status-at-risk",
        "delayed" => "status-badge status-delayed",
        "released" => "status-badge status-released",
        _ => "status-badge status-on-track",
    }
}

// Julian Day Number — exact integer day arithmetic, no external crate needed.
fn jdn(y: i32, m: i32, d: i32) -> i32 {
    let a = (14 - m) / 12;
    let y2 = y + 4800 - a;
    let m2 = m + 12 * a - 3;
    d + (153 * m2 + 2) / 5 + 365 * y2 + y2 / 4 - y2 / 100 + y2 / 400 - 32045
}

// Auto-pick "released" or "on_track" based on date vs today.
// Only used as a default — doesn't override "at_risk" / "delayed".
fn auto_status(release_date: &str) -> String {
    if let Some((ry, rm, rd)) = parse_date(release_date) {
        let (ty, tm, td) = today_ymd();
        if jdn(ry, rm, rd) <= jdn(ty, tm, td) {
            return "released".to_string();
        }
    }
    "on_track".to_string()
}

// Returns (label, css_modifier) for the countdown badge.
// modifier: "future" | "soon" | "today" | "overdue"
fn countdown(release: &str, ty: i32, tm: i32, td: i32) -> Option<(String, &'static str)> {
    let (ry, rm, rd) = parse_date(release)?;
    let delta = jdn(ry, rm, rd) - jdn(ty, tm, td);
    let label = if delta == 0 {
        "today".to_string()
    } else if delta > 0 {
        if delta < 14 {
            format!("in {delta}d")
        } else if delta < 90 {
            format!("in {}w", delta / 7)
        } else {
            format!("in ~{}mo", delta / 30)
        }
    } else {
        let ago = -delta;
        if ago < 14 {
            format!("{ago}d ago")
        } else if ago < 90 {
            format!("{}w ago", ago / 7)
        } else {
            format!("~{}mo ago", ago / 30)
        }
    };
    let modifier = if delta == 0 {
        "today"
    } else if delta > 0 && delta <= 30 {
        "soon"
    } else if delta > 30 {
        "future"
    } else {
        "overdue"
    };
    Some((label, modifier))
}

// ── Preset colors ─────────────────────────────────────────────────────────────

const PRESET_COLORS: &[&str] = &[
    "#2f81f7", "#3fb950", "#d29922", "#f85149", "#a371f7", "#39c5cf", "#e3b341", "#ff7b72",
    "#79c0ff", "#56d364",
];

// ── Month calendar sub-component ──────────────────────────────────────────────

// (id, color, comp_name, version, is_selected, is_actual, has_actual)
type DayEvent = (i64, String, String, String, bool, bool, bool);

#[component]
fn MonthCalendar(
    year: i32,
    month: i32,
    events: Vec<CalendarEvent>,
    components: Vec<CalendarComponent>,
    selected_event: Option<i64>,
    filter_component: Option<i64>,
    is_center: bool,
    on_event_click: EventHandler<i64>,
) -> Element {
    let first_dow = day_of_week(year, month, 1);
    let total_days = days_in_month(year, month);
    let (ty, tm, td) = today_ymd();
    let is_this_month = year == ty && month == tm;

    let day_data: Vec<(i32, bool, Vec<DayEvent>)> = (1..=total_days)
        .map(|day| {
            let is_today = is_this_month && day == td;
            let mut day_evs: Vec<DayEvent> = Vec::new();
            for ev in events.iter() {
                let comp_match = filter_component.map_or(true, |fc| ev.component_id == Some(fc));
                if !comp_match {
                    continue;
                }
                let comp = components.iter().find(|c| Some(c.id) == ev.component_id);
                let color = comp
                    .map(|c| c.color.clone())
                    .unwrap_or_else(|| "#8b949e".to_string());
                let name = comp
                    .map(|c| comp_display_name(c).to_string())
                    .unwrap_or_default();
                let is_sel = selected_event == Some(ev.id);
                let has_actual = ev.actual_release_date.is_some();
                let planned_day_match =
                    parse_date(&ev.release_date).map_or(false, |(_, _, d)| d == day);
                let actual_day_match = ev
                    .actual_release_date
                    .as_deref()
                    .and_then(|s| parse_date(s))
                    .map_or(false, |(_, _, d)| d == day);

                if planned_day_match {
                    // If actual also falls on the same day, show just one confirmed pill
                    let is_actual = actual_day_match;
                    day_evs.push((
                        ev.id,
                        color.clone(),
                        name.clone(),
                        ev.version.clone(),
                        is_sel,
                        is_actual,
                        has_actual,
                    ));
                } else if actual_day_match {
                    // Actual falls on this day but planned does not — show actual pill
                    day_evs.push((
                        ev.id,
                        color.clone(),
                        name.clone(),
                        ev.version.clone(),
                        is_sel,
                        true,
                        has_actual,
                    ));
                }
            }
            (day, is_today, day_evs)
        })
        .collect();

    rsx! {
        div { class: if is_center { "month-cal center" } else { "month-cal side" },
            div { class: "month-cal-header",
                span { class: "month-cal-title",
                    if is_center {
                        "{full_month_name(month)} {year}"
                    } else {
                        "{short_month_name(month)} {year}"
                    }
                }
            }
            div { class: "month-grid",
                for h in ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"] {
                    span { class: "month-dow", "{h}" }
                }
                for _ in 0..first_dow {
                    div { class: "month-day-empty" }
                }
                for (day, is_today, day_evs) in day_data {
                    div {
                        key: "{day}",
                        class: if is_today { "month-day today" } else { "month-day" },
                        span {
                            class: if is_today { "month-day-num today-circle" } else { "month-day-num" },
                            "{day}"
                        }
                        if !day_evs.is_empty() {
                            div { class: "month-day-events",
                                for (ev_id, color, comp_name, ev_version, is_sel, is_actual, has_actual) in day_evs {
                                    {
                                        let pill_class = if is_actual && is_sel {
                                            "cal-event-pill actual selected".to_string()
                                        } else if is_actual {
                                            "cal-event-pill actual".to_string()
                                        } else if is_sel && has_actual {
                                            "cal-event-pill planned-done selected".to_string()
                                        } else if is_sel {
                                            "cal-event-pill selected".to_string()
                                        } else if has_actual {
                                            "cal-event-pill planned-done".to_string()
                                        } else {
                                            "cal-event-pill".to_string()
                                        };
                                        let pill_key =
                                            format!("{ev_id}-{}", if is_actual { "a" } else { "p" });
                                        let label = if is_center {
                                            let prefix = if is_actual { "✓ " } else { "" };
                                            if !comp_name.is_empty() {
                                                format!("{prefix}{comp_name} {ev_version}")
                                            } else {
                                                format!("{prefix}{ev_version}")
                                            }
                                        } else {
                                            String::new()
                                        };
                                        rsx! {
                                            div {
                                                key: "{pill_key}",
                                                class: "{pill_class}",
                                                style: "background-color: {color};",
                                                title: "{comp_name}: {ev_version}",
                                                onclick: move |e: MouseEvent| {
                                                    e.stop_propagation();
                                                    on_event_click.call(ev_id);
                                                },
                                                if is_center {
                                                    span { class: "cal-event-label", "{label}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ── Component dialog ──────────────────────────────────────────────────────────

#[component]
fn ComponentDialog(
    component: Option<CalendarComponent>,
    on_confirm: EventHandler<CreateCalendarComponentInput>,
    on_close: EventHandler<()>,
) -> Element {
    let is_edit = component.is_some();
    let mut name = use_signal(|| {
        component
            .as_ref()
            .map(|c| c.name.clone())
            .unwrap_or_default()
    });
    let mut short_name = use_signal(|| {
        component
            .as_ref()
            .and_then(|c| c.short_name.clone())
            .unwrap_or_default()
    });
    let mut color = use_signal(|| {
        component
            .as_ref()
            .map(|c| c.color.clone())
            .unwrap_or_else(|| "#2f81f7".to_string())
    });

    rsx! {
        div { class: "modal-backdrop", onclick: move |_| on_close.call(()),
            div {
                class: "modal",
                style: "width: 400px;",
                onclick: move |e| e.stop_propagation(),
                div { class: "modal-header",
                    h2 { class: "modal-title",
                        if is_edit { "Edit component" } else { "Add component" }
                    }
                    button {
                        class: "icon-btn",
                        onclick: move |_| on_close.call(()),
                        Icon { width: 16, height: 16, icon: LdX }
                    }
                }
                form {
                    onsubmit: move |e| {
                        e.prevent_default();
                        let n = name.read().trim().to_string();
                        if n.is_empty() {
                            return;
                        }
                        let sn = short_name.read().trim().to_string();
                        on_confirm.call(CreateCalendarComponentInput {
                            name: n,
                            short_name: if sn.is_empty() { None } else { Some(sn) },
                            color: Some(color.read().clone()),
                        });
                    },
                    div { class: "field",
                        label { "Name" }
                        input {
                            class: "input",
                            required: true,
                            autofocus: true,
                            placeholder: "e.g. Envoy, Kubernetes, Gateway API",
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                        }
                    }
                    div { class: "field",
                        label {
                            "Short name "
                            span { class: "muted", style: "font-size:11px;", "(optional, shown on calendar pills)" }
                        }
                        input {
                            class: "input",
                            placeholder: "e.g. K8s, GW-API",
                            value: "{short_name}",
                            oninput: move |e| short_name.set(e.value()),
                        }
                    }
                    div { class: "field",
                        label { "Color" }
                        div { class: "color-presets",
                            for c in PRESET_COLORS {
                                button {
                                    key: "{c}",
                                    r#type: "button",
                                    class: if color.read().as_str() == *c {
                                        "color-preset active"
                                    } else {
                                        "color-preset"
                                    },
                                    style: "background-color: {c};",
                                    onclick: {
                                        let c = c.to_string();
                                        move |_| color.set(c.clone())
                                    },
                                }
                            }
                            input {
                                r#type: "color",
                                class: "color-input",
                                value: "{color}",
                                oninput: move |e| color.set(e.value()),
                            }
                        }
                    }
                    div { class: "form-actions",
                        button {
                            class: "btn btn-ghost",
                            r#type: "button",
                            onclick: move |_| on_close.call(()),
                            "Cancel"
                        }
                        button {
                            class: "btn btn-primary",
                            r#type: "submit",
                            if is_edit { "Save" } else { "Add component" }
                        }
                    }
                }
            }
        }
    }
}

// ── Release dialog ────────────────────────────────────────────────────────────

#[component]
fn ReleaseDialog(
    event: Option<CalendarEvent>,   // Some = edit mode
    prefill: Option<CalendarEvent>, // Some = duplicate mode (pre-filled new create)
    components: Vec<CalendarComponent>,
    on_confirm: EventHandler<CreateCalendarEventInput>,
    on_close: EventHandler<()>,
) -> Element {
    let is_edit = event.is_some();
    let is_dup = !is_edit && prefill.is_some();
    let init = event.as_ref().or(prefill.as_ref());

    let mut comp_id_str = use_signal(|| {
        init.and_then(|e| e.component_id)
            .map(|id| id.to_string())
            .unwrap_or_default()
    });
    let mut version = use_signal(|| init.map(|e| e.version.clone()).unwrap_or_default());
    let mut status = use_signal(|| {
        if is_edit {
            // Keep whatever was saved
            event
                .as_ref()
                .map(|e| e.status.clone())
                .unwrap_or_else(|| "on_track".to_string())
        } else {
            // New create or duplicate: auto-compute from the date
            auto_status(init.map(|e| e.release_date.as_str()).unwrap_or(""))
        }
    });
    let mut release_date = use_signal(|| init.map(|e| e.release_date.clone()).unwrap_or_default());
    // In duplicate mode don't copy actual date — user sets a fresh one.
    let mut actual_release_date = use_signal(|| {
        if is_edit {
            event
                .as_ref()
                .and_then(|e| e.actual_release_date.clone())
                .unwrap_or_default()
        } else {
            String::new()
        }
    });
    let mut note = use_signal(|| init.map(|e| e.note.clone()).unwrap_or_default());

    let title = if is_edit {
        "Edit release"
    } else if is_dup {
        "Duplicate release"
    } else {
        "Add release"
    };

    rsx! {
        div { class: "modal-backdrop", onclick: move |_| on_close.call(()),
            div {
                class: "modal",
                style: "width: 440px;",
                onclick: move |e| e.stop_propagation(),
                div { class: "modal-header",
                    h2 { class: "modal-title", "{title}" }
                    button {
                        class: "icon-btn",
                        onclick: move |_| on_close.call(()),
                        Icon { width: 16, height: 16, icon: LdX }
                    }
                }
                form {
                    onsubmit: move |e| {
                        e.prevent_default();
                        let v = version.read().trim().to_string();
                        let d = release_date.read().trim().to_string();
                        if v.is_empty() || d.is_empty() {
                            return;
                        }
                        let cid = comp_id_str.read().parse::<i64>().ok();
                        let st = status.read().clone();
                        let n = note.read().trim().to_string();
                        let ard = actual_release_date.read().trim().to_string();
                        on_confirm.call(CreateCalendarEventInput {
                            component_id: cid,
                            version: v,
                            status: Some(st),
                            release_date: d,
                            actual_release_date: if ard.is_empty() { None } else { Some(ard) },
                            note: if n.is_empty() { None } else { Some(n) },
                        });
                    },
                    div { class: "field",
                        label { "Component" }
                        select {
                            class: "select",
                            onchange: move |e| comp_id_str.set(e.value()),
                            option {
                                value: "",
                                selected: comp_id_str.read().is_empty(),
                                "— none —"
                            }
                            for c in components.iter() {
                                option {
                                    key: "{c.id}",
                                    value: "{c.id}",
                                    selected: *comp_id_str.read() == c.id.to_string(),
                                    "{c.name}"
                                }
                            }
                        }
                    }
                    div { class: "field",
                        label { "Version" }
                        input {
                            class: "input",
                            required: true,
                            autofocus: !is_edit,
                            placeholder: "e.g. 1.32.0",
                            value: "{version}",
                            oninput: move |e| version.set(e.value()),
                        }
                    }
                    div { class: "field",
                        label { "Status" }
                        div { class: "status-picker",
                            for (val, lbl, cls) in [
                                ("on_track", "On track", "status-btn status-on-track"),
                                ("at_risk",  "At risk",  "status-btn status-at-risk"),
                                ("delayed",  "Delayed",  "status-btn status-delayed"),
                                ("released", "Released", "status-btn status-released"),
                            ] {
                                button {
                                    key: "{val}",
                                    r#type: "button",
                                    class: if *status.read() == val {
                                        format!("{cls} active")
                                    } else {
                                        cls.to_string()
                                    },
                                    onclick: {
                                        let v = val.to_string();
                                        move |_| status.set(v.clone())
                                    },
                                    "{lbl}"
                                }
                            }
                        }
                    }
                    div { class: "field",
                        label { "Planned release date" }
                        input {
                            r#type: "date",
                            class: "input",
                            required: true,
                            value: "{release_date}",
                            oninput: move |e| {
                                let d = e.value();
                                // Auto-flip between on_track ↔ released based on date.
                                // Never clobber an explicit at_risk or delayed choice.
                                let cur = status.read().clone();
                                if cur == "on_track" || cur == "released" {
                                    status.set(auto_status(&d));
                                }
                                release_date.set(d);
                            },
                        }
                    }
                    div { class: "field",
                        label {
                            "Actual release date "
                            span { class: "muted", style: "font-size:11px;", "(optional)" }
                        }
                        input {
                            r#type: "date",
                            class: "input",
                            value: "{actual_release_date}",
                            oninput: move |e| actual_release_date.set(e.value()),
                        }
                    }
                    div { class: "field",
                        label { "Note (optional)" }
                        input {
                            class: "input",
                            placeholder: "e.g. LTS release, security patch",
                            value: "{note}",
                            oninput: move |e| note.set(e.value()),
                        }
                    }
                    div { class: "form-actions",
                        button {
                            class: "btn btn-ghost",
                            r#type: "button",
                            onclick: move |_| on_close.call(()),
                            "Cancel"
                        }
                        button {
                            class: "btn btn-primary",
                            r#type: "submit",
                            if is_edit { "Save" } else { "Add release" }
                        }
                    }
                }
            }
        }
    }
}

// ── Main page ─────────────────────────────────────────────────────────────────

#[component]
pub fn CalendarPage(id: i64) -> Element {
    let state = use_app_state();

    let mut id_sig = use_signal(|| id);
    if *id_sig.peek() != id {
        id_sig.set(id);
    }

    let detail = use_resource(move || {
        let _ = state.calendars_ver.read();
        let id = id_sig();
        async move { api::calendars::get(id).await }
    });

    let mut month_offset = use_signal(|| 0i32);
    let mut filter_component = use_signal(|| Option::<i64>::None);
    let mut selected_event = use_signal(|| Option::<i64>::None);

    let mut renaming = use_signal(|| false);
    let mut rename_value = use_signal(String::new);

    let mut show_comp_dialog = use_signal(|| false);
    let mut show_event_dialog = use_signal(|| false);
    let mut editing_component = use_signal(|| Option::<CalendarComponent>::None);
    let mut editing_event = use_signal(|| Option::<CalendarEvent>::None);
    let mut template_event = use_signal(|| Option::<CalendarEvent>::None);

    let detail_val = match detail.read().as_ref() {
        Some(Ok(d)) => Some(d.clone()),
        _ => None,
    };

    let Some(detail_val) = detail_val else {
        return rsx! {
            div { class: "empty-hint loading-text", "Loading calendar…" }
        };
    };

    let cal = detail_val.dashboard.clone();
    let components = detail_val.components.clone();
    let events = detail_val.events.clone();

    let (today_y, today_m, today_d) = today_ymd();
    let offset = *month_offset.read();
    let (center_y, center_m) = add_months(today_y, today_m, offset);
    let (prev_y, prev_m) = add_months(center_y, center_m, -1);
    let (next_y, next_m) = add_months(center_y, center_m, 1);

    // Include events with either planned or actual date in the given month.
    let events_for_month = |y: i32, m: i32| -> Vec<CalendarEvent> {
        events
            .iter()
            .filter(|ev| {
                let planned =
                    parse_date(&ev.release_date).map_or(false, |(ey, em, _)| ey == y && em == m);
                let actual = ev
                    .actual_release_date
                    .as_deref()
                    .and_then(|s| parse_date(s))
                    .map_or(false, |(ey, em, _)| ey == y && em == m);
                planned || actual
            })
            .cloned()
            .collect()
    };

    let prev_events = events_for_month(prev_y, prev_m);
    let center_events = events_for_month(center_y, center_m);
    let next_events = events_for_month(next_y, next_m);

    let sel_ev = *selected_event.read();
    let fil_comp = *filter_component.read();
    let cal_id = cal.id;
    let cal_name = cal.name.clone();

    let cal_name_onclick = cal_name.clone();
    let cal_name_blur = cal_name.clone();
    let cal_name_kd = cal_name.clone();

    rsx! {
        div { class: "cal-page",

            // ── Header ────────────────────────────────────────────────────────
            div { class: "page-header",
                div { class: "page-header-left",
                    Icon { width: 18, height: 18, icon: LdCalendar }
                    if renaming() {
                        input {
                            class: "input",
                            style: "font-size:18px; font-weight:600; width:280px;",
                            autofocus: true,
                            value: "{rename_value}",
                            oninput: move |e| rename_value.set(e.value()),
                            onblur: move |_| {
                                let name = rename_value.read().trim().to_string();
                                if !name.is_empty() && name != cal_name_blur {
                                    spawn(async move {
                                        let _ = api::calendars::update(
                                            cal_id,
                                            &UpdateCalendarDashboardInput {
                                                name: Some(name),
                                                ..Default::default()
                                            },
                                        )
                                        .await;
                                        state.invalidate_calendars();
                                    });
                                }
                                renaming.set(false);
                            },
                            onkeydown: move |e| {
                                if e.key() == Key::Enter {
                                    let name = rename_value.read().trim().to_string();
                                    if !name.is_empty() && name != cal_name_kd {
                                        spawn(async move {
                                            let _ = api::calendars::update(
                                                cal_id,
                                                &UpdateCalendarDashboardInput {
                                                    name: Some(name),
                                                    ..Default::default()
                                                },
                                            )
                                            .await;
                                            state.invalidate_calendars();
                                        });
                                    }
                                    renaming.set(false);
                                } else if e.key() == Key::Escape {
                                    renaming.set(false);
                                }
                            },
                        }
                    } else {
                        h1 {
                            class: "page-title editable",
                            onclick: move |_| {
                                rename_value.set(cal_name_onclick.clone());
                                renaming.set(true);
                            },
                            "{cal_name}"
                        }
                    }
                }
                div { class: "toolbar",
                    if offset != 0 {
                        button {
                            class: "cal-today-btn",
                            title: "Go to current month",
                            onclick: move |_| month_offset.set(0),
                            "Today"
                        }
                    }
                    button {
                        class: "btn btn-ghost",
                        onclick: move |_| {
                            editing_component.set(None);
                            show_comp_dialog.set(true);
                        },
                        Icon { width: 15, height: 15, icon: LdPlus }
                        "Component"
                    }
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| {
                            editing_event.set(None);
                            template_event.set(None);
                            show_event_dialog.set(true);
                        },
                        Icon { width: 15, height: 15, icon: LdPlus }
                        "Add Release"
                    }
                }
            }

            // ── 3-month calendar strip ────────────────────────────────────────
            div { class: "cal-strip-wrapper",
                button {
                    class: "cal-nav-btn",
                    title: "Previous month",
                    onclick: move |_| *month_offset.write() -= 1,
                    Icon { width: 18, height: 18, icon: LdChevronLeft }
                }
                div { class: "cal-months-strip",
                    MonthCalendar {
                        year: prev_y,
                        month: prev_m,
                        events: prev_events,
                        components: components.clone(),
                        selected_event: sel_ev,
                        filter_component: fil_comp,
                        is_center: false,
                        on_event_click: move |ev_id: i64| {
                            selected_event.set(Some(ev_id));
                            *month_offset.write() -= 1;
                        },
                    }
                    MonthCalendar {
                        year: center_y,
                        month: center_m,
                        events: center_events,
                        components: components.clone(),
                        selected_event: sel_ev,
                        filter_component: fil_comp,
                        is_center: true,
                        on_event_click: move |ev_id: i64| {
                            selected_event.set(Some(ev_id));
                        },
                    }
                    MonthCalendar {
                        year: next_y,
                        month: next_m,
                        events: next_events,
                        components: components.clone(),
                        selected_event: sel_ev,
                        filter_component: fil_comp,
                        is_center: false,
                        on_event_click: move |ev_id: i64| {
                            selected_event.set(Some(ev_id));
                            *month_offset.write() += 1;
                        },
                    }
                }
                button {
                    class: "cal-nav-btn",
                    title: "Next month",
                    onclick: move |_| *month_offset.write() += 1,
                    Icon { width: 18, height: 18, icon: LdChevronRight }
                }
            }

            // ── Components + Events ───────────────────────────────────────────
            div { class: "cal-body",

                // Component filter chips
                if !components.is_empty() {
                    div { class: "component-chips",
                        for comp in components.iter() {
                            {
                                let comp_id = comp.id;
                                let comp_color = comp.color.clone();
                                let comp_name = comp.name.clone();
                                let is_active = fil_comp == Some(comp_id);
                                let comp_clone = comp.clone();
                                let events_clone = events.clone();
                                rsx! {
                                    div {
                                        key: "{comp_id}",
                                        class: if is_active { "component-chip active" } else { "component-chip" },
                                        style: if is_active {
                                            format!("border-color: {comp_color}; background: color-mix(in srgb, {comp_color} 15%, var(--panel));")
                                        } else {
                                            "border-color: var(--border);".to_string()
                                        },
                                        onclick: move |_| {
                                            let currently = *filter_component.read();
                                            if currently == Some(comp_id) {
                                                filter_component.set(None);
                                            } else {
                                                filter_component.set(Some(comp_id));
                                                let (ty, tm, td) = today_ymd();
                                                let today_abs = ty * 10000 + tm * 100 + td;
                                                let future = events_clone
                                                    .iter()
                                                    .filter(|ev| ev.component_id == Some(comp_id))
                                                    .filter_map(|ev| parse_date(&ev.release_date))
                                                    .filter(|(y, m, d)| {
                                                        y * 10000 + m * 100 + d >= today_abs
                                                    })
                                                    .min_by_key(|(y, m, d)| y * 10000 + m * 100 + d);
                                                let target = future.or_else(|| {
                                                    events_clone
                                                        .iter()
                                                        .filter(|ev| ev.component_id == Some(comp_id))
                                                        .filter_map(|ev| parse_date(&ev.release_date))
                                                        .max_by_key(|(y, m, d)| y * 10000 + m * 100 + d)
                                                });
                                                if let Some((ey, em, _)) = target {
                                                    *month_offset.write() =
                                                        ey * 12 + em - (today_y * 12 + today_m);
                                                }
                                            }
                                        },
                                        span { class: "chip-dot", style: "background-color: {comp_color};" }
                                        span { "{comp_name}" }
                                        button {
                                            class: "chip-action-btn",
                                            title: "Edit component",
                                            onclick: move |e: MouseEvent| {
                                                e.stop_propagation();
                                                editing_component.set(Some(comp_clone.clone()));
                                                show_comp_dialog.set(true);
                                            },
                                            Icon { width: 11, height: 11, icon: LdPencil }
                                        }
                                        button {
                                            class: "chip-action-btn",
                                            title: "Delete component",
                                            onclick: move |e: MouseEvent| {
                                                e.stop_propagation();
                                                spawn(async move {
                                                    let _ =
                                                        api::calendars::delete_component(comp_id).await;
                                                    state.invalidate_calendars();
                                                });
                                            },
                                            Icon { width: 11, height: 11, icon: LdX }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Events list
                div { class: "cal-events",
                    div { class: "cal-events-header",
                        span { class: "cal-events-title", "Releases" }
                        {
                            let shown = events
                                .iter()
                                .filter(|ev| fil_comp.map_or(true, |fc| ev.component_id == Some(fc)))
                                .count();
                            rsx! { span { class: "muted", style: "font-size:12px;", " ({shown})" } }
                        }
                    }
                    {
                        let visible: Vec<&CalendarEvent> = events
                            .iter()
                            .filter(|ev| fil_comp.map_or(true, |fc| ev.component_id == Some(fc)))
                            .collect();
                        if visible.is_empty() {
                            rsx! {
                                div { class: "empty-hint",
                                    if fil_comp.is_some() {
                                        "No releases for this component. Click \"+ Add Release\" to add one."
                                    } else {
                                        "No releases yet. Click \"+ Add Release\" to add one."
                                    }
                                }
                            }
                        } else {
                            rsx! {
                                for ev in visible {
                                    {
                                        let comp =
                                            components.iter().find(|c| Some(c.id) == ev.component_id);
                                        let color = comp
                                            .map(|c| c.color.clone())
                                            .unwrap_or_else(|| "#8b949e".to_string());
                                        let comp_name = comp
                                            .map(|c| c.name.clone())
                                            .unwrap_or_else(|| "—".to_string());
                                        let ev_id = ev.id;
                                        let ev_version = ev.version.clone();
                                        let ev_date = ev.release_date.clone();
                                        let is_sel = sel_ev == Some(ev_id);
                                        let ev_for_edit = ev.clone();
                                        let ev_for_dup = ev.clone();
                                        // Countdown only shown when no actual date recorded yet
                                        let cd = if ev.actual_release_date.is_none() {
                                            countdown(&ev.release_date, today_y, today_m, today_d)
                                        } else {
                                            None
                                        };
                                        let note_html = if !ev.note.is_empty() {
                                            Some(md_to_html(&ev.note))
                                        } else {
                                            None
                                        };
                                        rsx! {
                                            div {
                                                key: "{ev_id}",
                                                class: if is_sel {
                                                    "event-row selected"
                                                } else {
                                                    "event-row"
                                                },
                                                onclick: move |_| {
                                                    selected_event.set(Some(ev_id));
                                                    if let Some((ey, em, _)) = parse_date(&ev_date) {
                                                        *month_offset.write() =
                                                            ey * 12 + em - (today_y * 12 + today_m);
                                                    }
                                                },
                                                // ── Main info row ──────────────────────────
                                                div { class: "event-row-main",
                                                    div { class: "event-comp",
                                                        span {
                                                            class: "event-dot",
                                                            style: "background-color: {color};",
                                                        }
                                                        span { class: "event-comp-name", "{comp_name}" }
                                                    }
                                                    span { class: "event-version", "{ev_version}" }
                                                    span {
                                                        class: "{status_css(&ev.status)}",
                                                        "{status_label(&ev.status)}"
                                                    }
                                                    div { class: "event-dates",
                                                        span { class: "event-date muted", "{ev.release_date}" }
                                                        if let Some(ard) = &ev.actual_release_date {
                                                            span { class: "event-date-actual", "✓ {ard}" }
                                                        }
                                                    }
                                                    if let Some((label, modifier)) = cd {
                                                        span {
                                                            class: "countdown-badge countdown-{modifier}",
                                                            "{label}"
                                                        }
                                                    }
                                                    div { class: "event-actions",
                                                    button {
                                                        class: "icon-btn",
                                                        title: "Duplicate",
                                                        onclick: move |e: MouseEvent| {
                                                            e.stop_propagation();
                                                            template_event
                                                                .set(Some(ev_for_dup.clone()));
                                                            editing_event.set(None);
                                                            show_event_dialog.set(true);
                                                        },
                                                        Icon { width: 13, height: 13, icon: LdCopy }
                                                    }
                                                    button {
                                                        class: "icon-btn",
                                                        title: "Edit",
                                                        onclick: move |e: MouseEvent| {
                                                            e.stop_propagation();
                                                            editing_event
                                                                .set(Some(ev_for_edit.clone()));
                                                            template_event.set(None);
                                                            show_event_dialog.set(true);
                                                        },
                                                        Icon { width: 13, height: 13, icon: LdPencil }
                                                    }
                                                    button {
                                                        class: "icon-btn danger",
                                                        title: "Delete",
                                                        onclick: move |e: MouseEvent| {
                                                            e.stop_propagation();
                                                            spawn(async move {
                                                                let _ =
                                                                    api::calendars::delete_event(ev_id)
                                                                        .await;
                                                                state.invalidate_calendars();
                                                            });
                                                        },
                                                        Icon {
                                                            width: 13,
                                                            height: 13,
                                                            icon: LdTrash2,
                                                        }
                                                    }
                                                }
                                                } // close event-row-main
                                                // ── Markdown note ──────────────────────────
                                                if let Some(html) = note_html {
                                                    div {
                                                        class: "event-note-md",
                                                        dangerous_inner_html: "{html}",
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // ── Dialogs ───────────────────────────────────────────────────────────
        if show_comp_dialog() {
            ComponentDialog {
                component: editing_component.read().clone(),
                on_confirm: move |input: CreateCalendarComponentInput| {
                    let comp_id_opt = editing_component.read().as_ref().map(|c| c.id);
                    if let Some(cid) = comp_id_opt {
                        let name = input.name.clone();
                        let short_name = input.short_name.clone();
                        let color = input.color.clone();
                        spawn(async move {
                            let _ = api::calendars::update_component(
                                cid,
                                &UpdateCalendarComponentInput {
                                    name: Some(name),
                                    short_name,
                                    color,
                                    position: None,
                                },
                            )
                            .await;
                            state.invalidate_calendars();
                        });
                    } else {
                        spawn(async move {
                            let _ = api::calendars::create_component(cal_id, &input).await;
                            state.invalidate_calendars();
                        });
                    }
                    show_comp_dialog.set(false);
                    editing_component.set(None);
                },
                on_close: move |_| {
                    show_comp_dialog.set(false);
                    editing_component.set(None);
                },
            }
        }
        if show_event_dialog() {
            ReleaseDialog {
                event: editing_event.read().clone(),
                prefill: template_event.read().clone(),
                components: components.clone(),
                on_confirm: move |input: CreateCalendarEventInput| {
                    let ev_id_opt = editing_event.read().as_ref().map(|e| e.id);
                    if let Some(eid) = ev_id_opt {
                        let version = input.version.clone();
                        let status = input.status.clone();
                        let release_date = input.release_date.clone();
                        let actual_release_date = input.actual_release_date.clone();
                        let note = input.note.clone();
                        let component_id = input.component_id;
                        spawn(async move {
                            let _ = api::calendars::update_event(
                                eid,
                                &UpdateCalendarEventInput {
                                    component_id,
                                    version: Some(version),
                                    status,
                                    release_date: Some(release_date),
                                    actual_release_date,
                                    note: Some(note.unwrap_or_default()),
                                },
                            )
                            .await;
                            state.invalidate_calendars();
                        });
                    } else {
                        spawn(async move {
                            let _ = api::calendars::create_event(cal_id, &input).await;
                            state.invalidate_calendars();
                        });
                    }
                    show_event_dialog.set(false);
                    editing_event.set(None);
                    template_event.set(None);
                },
                on_close: move |_| {
                    show_event_dialog.set(false);
                    editing_event.set(None);
                    template_event.set(None);
                },
            }
        }
    }
}
