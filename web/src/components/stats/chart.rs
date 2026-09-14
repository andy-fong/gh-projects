//! Hand-rolled inline SVG charting.
//!
//! No charting crate on purpose: the dataset is ~6-26 weeks x a few series, and
//! a crate like `plotters` would add hundreds of KB to the WASM bundle and
//! render to `<canvas>` — losing DOM hover, CSS-variable theming, and requiring
//! a `ResizeObserver`. A `polyline points="..."` string costs ~150 lines.

use dioxus::prelude::*;
use std::collections::BTreeSet;

use crate::datetime::week_label;

#[derive(Clone, PartialEq)]
pub struct ChartSeries {
    pub name: String,
    /// A CSS colour — `var(--good)` and friends work directly as SVG paint.
    pub color: String,
    pub values: Vec<f64>,
}

const VB_W: f64 = 900.0;
const M_L: f64 = 34.0;
const M_R: f64 = 12.0;
const M_T: f64 = 10.0;
const M_B: f64 = 22.0;

/// Round a raw max up to a "nice" axis top: 1/2/5 x 10^k, with 4 ticks.
///
/// Floored at 4 so the four gridline labels are always distinct integers — a
/// quiet week would otherwise produce a top of 1 and render "0 0 1 1 1".
fn nice_top(raw: f64) -> f64 {
    let raw = raw.max(1.0);
    let target = raw / 4.0;
    let mag = 10f64.powf(target.log10().floor());
    let step = [1.0, 2.0, 5.0, 10.0]
        .iter()
        .map(|m| m * mag)
        .find(|s| *s >= target)
        .unwrap_or(10.0 * mag);
    ((raw / step).ceil() * step).max(4.0)
}

fn x_at(i: usize, n: usize) -> f64 {
    let pw = VB_W - M_L - M_R;
    if n <= 1 {
        M_L + pw / 2.0
    } else {
        M_L + pw * i as f64 / (n - 1) as f64
    }
}

fn y_at(v: f64, top: f64, vb_h: f64) -> f64 {
    let ph = vb_h - M_T - M_B;
    M_T + ph * (1.0 - (v / top).clamp(0.0, 1.0))
}

#[component]
pub fn TrendChart(weeks: Vec<String>, series: Vec<ChartSeries>, height: u32) -> Element {
    let mut hover = use_signal(|| None::<usize>);
    let mut hidden = use_signal(BTreeSet::<String>::new);

    let n = weeks.len();
    if n == 0 {
        return rsx! { div { class: "empty-hint", "No data in this window yet." } };
    }
    let vb_h = height as f64;

    let visible: Vec<ChartSeries> = series
        .iter()
        .filter(|s| !hidden.read().contains(&s.name))
        .cloned()
        .collect();
    let top = nice_top(
        visible
            .iter()
            .flat_map(|s| s.values.iter().copied())
            .fold(0.0f64, f64::max),
    );

    let ticks: Vec<(f64, String)> = (0..=4)
        .map(|k| {
            let v = top * k as f64 / 4.0;
            (y_at(v, top, vb_h), format!("{v:.0}"))
        })
        .collect();

    // Thin the x labels so ~26 weeks don't collide.
    let lstep = (n / 13).max(1);
    let xlabels: Vec<(f64, String)> = weeks
        .iter()
        .enumerate()
        .filter(|(i, _)| i % lstep == 0)
        .map(|(i, w)| (x_at(i, n), week_label(w)))
        .collect();

    let paths: Vec<(ChartSeries, String)> = visible
        .iter()
        .map(|s| {
            let pts = s
                .values
                .iter()
                .enumerate()
                .map(|(i, v)| format!("{:.1},{:.1}", x_at(i, n), y_at(*v, top, vb_h)))
                .collect::<Vec<_>>()
                .join(" ");
            (s.clone(), pts)
        })
        .collect();

    let band = (VB_W - M_L - M_R) / n as f64;

    rsx! {
        div { class: "ts-chart-wrap",
            svg {
                class: "ts-chart",
                view_box: "0 0 {VB_W} {vb_h}",
                width: "100%",
                height: "{height}",

                for (y, label) in ticks.iter() {
                    line {
                        x1: "{M_L}", x2: "{VB_W - M_R}", y1: "{y}", y2: "{y}",
                        stroke: "var(--border-soft)", stroke_width: "1",
                    }
                    text {
                        x: "{M_L - 6.0}", y: "{y + 3.5}",
                        text_anchor: "end", font_size: "10", fill: "var(--muted-dim)",
                        "{label}"
                    }
                }

                for (x, label) in xlabels.iter() {
                    text {
                        x: "{x}", y: "{vb_h - 6.0}",
                        text_anchor: "middle", font_size: "10", fill: "var(--muted-dim)",
                        "{label}"
                    }
                }

                if let Some(i) = hover() {
                    line {
                        x1: "{x_at(i, n)}", x2: "{x_at(i, n)}",
                        y1: "{M_T}", y2: "{vb_h - M_B}",
                        stroke: "var(--muted-dim)", stroke_width: "1", stroke_dasharray: "3 3",
                    }
                }

                for (s, pts) in paths.iter() {
                    polyline {
                        points: "{pts}",
                        fill: "none",
                        stroke: "{s.color}",
                        stroke_width: "2",
                        stroke_linejoin: "round",
                        stroke_linecap: "round",
                    }
                    // Dots are filled with the page background and ringed in the
                    // series colour, so crossing lines stay readable.
                    for (i, v) in s.values.iter().enumerate() {
                        circle {
                            cx: "{x_at(i, n)}",
                            cy: "{y_at(*v, top, vb_h)}",
                            r: if hover() == Some(i) { "4" } else { "2.5" },
                            fill: "var(--bg)",
                            stroke: "{s.color}",
                            stroke_width: "1.5",
                        }
                    }
                }

                // Invisible hover bands, last so they sit on top.
                for i in 0..n {
                    rect {
                        x: "{M_L + band * i as f64}", y: "{M_T}",
                        width: "{band}", height: "{vb_h - M_T - M_B}",
                        fill: "transparent",
                        onmouseenter: move |_| hover.set(Some(i)),
                        onmouseleave: move |_| hover.set(None),
                    }
                }
            }

            // Tooltip is HTML over the SVG, positioned in % so it needs no
            // pixel measurement of the fluid-width chart.
            if let Some(i) = hover() {
                div {
                    class: "ts-chart-tip",
                    style: {
                        let pct = x_at(i, n) / VB_W * 100.0;
                        if pct > 62.0 {
                            format!("right:{:.1}%;", 100.0 - pct + 1.0)
                        } else {
                            format!("left:{:.1}%;", pct + 1.0)
                        }
                    },
                    div { class: "ts-chart-tip-head", "Week of {week_label(&weeks[i])}" }
                    for s in visible.iter() {
                        div { class: "ts-chart-tip-row",
                            span { class: "ts-swatch", style: "background:{s.color};" }
                            span { "{s.name}" }
                            span { class: "ts-chart-tip-val mono", "{s.values[i]:.0}" }
                        }
                    }
                }
            }
        }

        div { class: "ts-legend",
            for s in series.iter() {
                {
                    let name = s.name.clone();
                    let off = hidden.read().contains(&s.name);
                    rsx! {
                        button {
                            key: "{s.name}",
                            class: if off { "ts-legend-item off" } else { "ts-legend-item" },
                            onclick: move |_| {
                                let mut h = hidden.write();
                                if !h.remove(&name) { h.insert(name.clone()); }
                            },
                            span { class: "ts-swatch", style: "background:{s.color};" }
                            "{s.name}"
                        }
                    }
                }
            }
        }
    }
}

/// A tiny trend line for a table row. `preserve_aspect_ratio: none` plus
/// `vector_effect: non-scaling-stroke` keeps the stroke even when the viewBox
/// is squashed horizontally.
///
/// `max` is supplied by the caller rather than taken per-row: self-normalising
/// would draw someone reviewing 1 PR a week at the same height as someone
/// reviewing 20, which is exactly the comparison this column exists to make.
#[component]
pub fn Sparkline(values: Vec<f64>, color: String, max: f64) -> Element {
    let n = values.len();
    if n == 0 {
        return rsx! { span { class: "muted text-xs", "—" } };
    }
    let max = max.max(1.0);
    let pts = values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let x = if n <= 1 { 50.0 } else { 100.0 * i as f64 / (n - 1) as f64 };
            format!("{:.1},{:.1}", x, 18.0 - 16.0 * (v / max))
        })
        .collect::<Vec<_>>()
        .join(" ");
    rsx! {
        svg {
            class: "ts-spark",
            view_box: "0 0 100 20",
            width: "84", height: "18",
            preserve_aspect_ratio: "none",
            polyline {
                points: "{pts}",
                fill: "none",
                stroke: "{color}",
                stroke_width: "1.5",
                stroke_linejoin: "round",
                vector_effect: "non-scaling-stroke",
            }
        }
    }
}
