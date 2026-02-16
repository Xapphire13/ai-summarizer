use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use maud::{Markup, html};

const WIDTH: f64 = 600.0;
const HEIGHT: f64 = 200.0;
const MARGIN_LEFT: f64 = 60.0;
const MARGIN_RIGHT: f64 = 20.0;
const MARGIN_TOP: f64 = 20.0;
const MARGIN_BOTTOM: f64 = 30.0;

fn format_time(ts: DateTime<Utc>) -> String {
    ts.format("%H:%M").to_string()
}

fn format_value(v: f64) -> String {
    if v == v.floor() && v.abs() < 1_000_000.0 {
        format!("{v:.0}")
    } else {
        format!("{v:.1}")
    }
}

pub fn render_uptime_chart(
    heartbeats: &VecDeque<DateTime<Utc>>,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    num_buckets: usize,
) -> Markup {
    let total_secs = (end - start).num_seconds().max(1);
    let bucket_secs = (total_secs / num_buckets as i64).max(1);
    let actual_buckets = (total_secs / bucket_secs) as usize;

    let mut has_heartbeat = vec![false; actual_buckets];
    for ts in heartbeats {
        let offset = (*ts - start).num_seconds();
        if offset < 0 {
            continue;
        }
        let idx = (offset / bucket_secs) as usize;
        if idx < actual_buckets {
            has_heartbeat[idx] = true;
        }
    }

    let any_heartbeats = heartbeats.iter().any(|ts| *ts >= start && *ts <= end);

    let chart_h = 30.0;
    let total_h = chart_h + MARGIN_BOTTOM + 10.0;
    let chart_w = WIDTH - MARGIN_LEFT - MARGIN_RIGHT;
    let bar_w = chart_w / actual_buckets as f64;

    let label_y = 10.0 + chart_h + 18.0;
    let mid = start + chrono::Duration::seconds(total_secs / 2);
    let mid_x = MARGIN_LEFT + chart_w / 2.0;
    let end_x = MARGIN_LEFT + chart_w;

    html! {
        svg viewBox=(format!("0 0 {WIDTH} {total_h}")) xmlns="http://www.w3.org/2000/svg" style="width:100%;height:auto" {
            rect width=(WIDTH) height=(total_h) style="fill: var(--background)" {}
            @for (i, &has) in has_heartbeat.iter().enumerate() {
                @let x = MARGIN_LEFT + i as f64 * bar_w;
                @let color_var = if !any_heartbeats {
                    "var(--muted)"
                } else if has {
                    "var(--status-online)"
                } else {
                    "var(--status-offline)"
                };
                rect x=(x) y="10" width=((bar_w - 1.0).max(0.5)) height=(chart_h) opacity="0.8" style=(format!("fill: {color_var}")) {}
            }
            text x=(MARGIN_LEFT) y=(label_y) font-size="11" text-anchor="start" style="fill: var(--foreground); font-family: inherit" {
                (format_time(start))
            }
            text x=(mid_x) y=(label_y) font-size="11" text-anchor="middle" style="fill: var(--foreground); font-family: inherit" {
                (format_time(mid))
            }
            text x=(end_x) y=(label_y) font-size="11" text-anchor="end" style="fill: var(--foreground); font-family: inherit" {
                (format_time(end))
            }
        }
    }
}

pub fn render_bar_chart(buckets: &[(DateTime<Utc>, f64)], label: &str) -> Markup {
    if buckets.is_empty() {
        return empty_chart(label);
    }

    let max_val = buckets.iter().map(|(_, v)| *v).fold(0.0_f64, f64::max);
    let max_val = if max_val == 0.0 { 1.0 } else { max_val };

    let chart_w = WIDTH - MARGIN_LEFT - MARGIN_RIGHT;
    let chart_h = HEIGHT - MARGIN_TOP - MARGIN_BOTTOM;
    let bar_w = chart_w / buckets.len() as f64;

    html! {
        svg viewBox=(format!("0 0 {WIDTH} {HEIGHT}")) xmlns="http://www.w3.org/2000/svg" style="width:100%;height:auto" {
            rect width=(WIDTH) height=(HEIGHT) style="fill: var(--background)" {}
            text x=(MARGIN_LEFT) y="14" font-size="12" style="fill: var(--foreground); font-family: inherit" { (label) }
            text x=(MARGIN_LEFT - 5.0) y=(MARGIN_TOP + 10.0) font-size="10" text-anchor="end" style="fill: var(--foreground); font-family: inherit" {
                (format_value(max_val))
            }
            text x=(MARGIN_LEFT - 5.0) y=(MARGIN_TOP + chart_h) font-size="10" text-anchor="end" style="fill: var(--foreground); font-family: inherit" { "0" }
            @for (i, (ts, val)) in buckets.iter().enumerate() {
                @let bar_h = (val / max_val) * chart_h;
                @let x = MARGIN_LEFT + i as f64 * bar_w;
                @let y = MARGIN_TOP + chart_h - bar_h;
                rect x=(x) y=(y) width=((bar_w - 1.0).max(0.5)) height=(bar_h) opacity="0.7" style="fill: var(--foreground)" {
                    title { (format_time(*ts)) ": " (format_value(*val)) }
                }
            }
            (write_x_axis(buckets, chart_w))
        }
    }
}

pub fn render_line_chart(buckets: &[(DateTime<Utc>, f64)], label: &str) -> Markup {
    if buckets.is_empty() {
        return empty_chart(label);
    }

    let max_val = buckets.iter().map(|(_, v)| *v).fold(0.0_f64, f64::max);
    let min_val = buckets
        .iter()
        .map(|(_, v)| *v)
        .fold(f64::INFINITY, f64::min);
    let range = if (max_val - min_val).abs() < f64::EPSILON {
        1.0
    } else {
        max_val - min_val
    };

    let chart_w = WIDTH - MARGIN_LEFT - MARGIN_RIGHT;
    let chart_h = HEIGHT - MARGIN_TOP - MARGIN_BOTTOM;

    let mut points = String::new();
    for (i, (_, val)) in buckets.iter().enumerate() {
        let x = MARGIN_LEFT + (i as f64 / (buckets.len() - 1).max(1) as f64) * chart_w;
        let y = MARGIN_TOP + chart_h - ((val - min_val) / range) * chart_h;
        if !points.is_empty() {
            points.push(' ');
        }
        use std::fmt::Write;
        let _ = write!(points, "{x},{y}");
    }

    html! {
        svg viewBox=(format!("0 0 {WIDTH} {HEIGHT}")) xmlns="http://www.w3.org/2000/svg" style="width:100%;height:auto" {
            rect width=(WIDTH) height=(HEIGHT) style="fill: var(--background)" {}
            text x=(MARGIN_LEFT) y="14" font-size="12" style="fill: var(--foreground); font-family: inherit" { (label) }
            text x=(MARGIN_LEFT - 5.0) y=(MARGIN_TOP + 10.0) font-size="10" text-anchor="end" style="fill: var(--foreground); font-family: inherit" {
                (format_value(max_val))
            }
            text x=(MARGIN_LEFT - 5.0) y=(MARGIN_TOP + chart_h) font-size="10" text-anchor="end" style="fill: var(--foreground); font-family: inherit" {
                (format_value(min_val))
            }
            polyline points=(points) fill="none" stroke-width="2" style="stroke: var(--foreground)" {}
            @for (i, (ts, val)) in buckets.iter().enumerate() {
                @let x = MARGIN_LEFT + (i as f64 / (buckets.len() - 1).max(1) as f64) * chart_w;
                @let y = MARGIN_TOP + chart_h - ((val - min_val) / range) * chart_h;
                circle cx=(x) cy=(y) r="3" style="fill: var(--foreground)" {
                    title { (format_time(*ts)) ": " (format_value(*val)) }
                }
            }
            (write_x_axis(buckets, chart_w))
        }
    }
}

fn write_x_axis(buckets: &[(DateTime<Utc>, f64)], chart_w: f64) -> Markup {
    let label_y = HEIGHT - 5.0;
    html! {
        @if let Some((ts, _)) = buckets.first() {
            text x=(MARGIN_LEFT) y=(label_y) font-size="11" text-anchor="start" style="fill: var(--foreground); font-family: inherit" {
                (format_time(*ts))
            }
        }
        @if buckets.len() > 2 {
            @let mid = buckets.len() / 2;
            @let mid_x = MARGIN_LEFT + chart_w / 2.0;
            text x=(mid_x) y=(label_y) font-size="11" text-anchor="middle" style="fill: var(--foreground); font-family: inherit" {
                (format_time(buckets[mid].0))
            }
        }
        @if let Some((ts, _)) = buckets.last() {
            @let end_x = MARGIN_LEFT + chart_w;
            text x=(end_x) y=(label_y) font-size="11" text-anchor="end" style="fill: var(--foreground); font-family: inherit" {
                (format_time(*ts))
            }
        }
    }
}

fn empty_chart(label: &str) -> Markup {
    html! {
        svg viewBox=(format!("0 0 {WIDTH} {HEIGHT}")) xmlns="http://www.w3.org/2000/svg" style="width:100%;height:auto" {
            rect width=(WIDTH) height=(HEIGHT) style="fill: var(--background)" {}
            text x=(WIDTH / 2.0) y=(HEIGHT / 2.0) font-size="14" text-anchor="middle" style="fill: var(--foreground); font-family: inherit" {
                (label) " — no data"
            }
        }
    }
}
