use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use maud::{Markup, html};

use crate::charts::{self, svg};
use crate::config::{CHART_BUCKET_COUNT, MIN_BUCKET_SECONDS, ONLINE_GRACE_PERIOD};
use crate::dashboard_config::{self, ChartConfig, ChartType};
use crate::state::AppState;
use crate::styles::Charts as ChartClass;

use super::WindowQuery;
use super::breadcrumbs::{Breadcrumb, breadcrumbs};
use super::page_shell;

/// Available time windows for chart display: `(query_key, display_label, seconds)`.
/// The largest window ("7d") matches the maximum metric retention period.
const TIME_WINDOWS: &[(&str, &str, i64)] = &[
    ("1h", "1h", 3600),
    ("6h", "6h", 3600 * 6),
    ("12h", "12h", 3600 * 12),
    ("1d", "1d", 86400),
    ("2d", "2d", 86400 * 2),
    ("3d", "3d", 86400 * 3),
    ("7d", "7d (all)", 86400 * 7),
];

/// Fallback window key used when no `?window=` query param is provided.
const DEFAULT_WINDOW: &str = "1d";

/// Resolves a `?window=` query param to `(seconds, key)`, falling back to [`DEFAULT_WINDOW`].
fn parse_window(window: Option<&str>) -> (i64, &str) {
    let key = window.unwrap_or(DEFAULT_WINDOW);
    for &(k, _, secs) in TIME_WINDOWS {
        if k == key {
            return (secs, k);
        }
    }
    (86400, DEFAULT_WINDOW)
}

fn format_relative(seconds_ago: i64) -> String {
    if seconds_ago < 60 {
        format!("{seconds_ago}s ago")
    } else if seconds_ago < 3600 {
        format!("{}m ago", seconds_ago / 60)
    } else if seconds_ago < 86400 {
        format!("{}h ago", seconds_ago / 3600)
    } else {
        format!("{}d ago", seconds_ago / 86400)
    }
}

pub async fn bot_detail(
    Path(name): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Markup, StatusCode> {
    let registry = state.registry.read().unwrap();
    let bot = registry.get(&name).ok_or(StatusCode::NOT_FOUND)?;
    let online = registry.is_online(&name, ONLINE_GRACE_PERIOD);
    let ago = (Utc::now() - bot.last_heartbeat).num_seconds();
    let bot_name = bot.name.clone();
    drop(registry);

    let content = html! {
        (breadcrumbs(&[
            Breadcrumb { label: "bots", href: Some("/")},
            Breadcrumb { label: &name, href: None }])
        )

        @if online {
            div.status.online { "[ONLINE]" }
        } @else {
            div.status.offline { "[OFFLINE]" }
        }
        div.meta { "Last seen: " (format_relative(ago)) }

        div #charts-container
            hx-get=(format!("/fragments/bot/{bot_name}/charts"))
            hx-trigger="every 60s"
            hx-swap="innerHTML"
        {
            (render_charts(&name, None, &state))
        }
    };
    Ok(page_shell(&format!("{bot_name} | Dashboard"), content))
}

pub async fn fragment_bot_charts(
    Path(name): Path<String>,
    Query(query): Query<WindowQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<Markup, StatusCode> {
    Ok(render_charts(&name, query.window.as_deref(), &state))
}

pub fn render_charts(name: &str, window: Option<&str>, state: &Arc<AppState>) -> Markup {
    let (window_secs, active_window) = parse_window(window);
    let now = Utc::now();
    let start = now - chrono::Duration::seconds(window_secs);
    let end = now;

    let time_selector = render_time_selector(name, active_window);
    let uptime_section = render_uptime_section(state, name, start, end);

    let config = match dashboard_config::load(name) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("warning: failed to load dashboard config for {name}: {e}");
            dashboard_config::DashboardConfig::default()
        }
    };
    let metrics_guard = state.metrics.read().unwrap();

    let chart_markup: Vec<Markup> = config
        .charts
        .iter()
        .enumerate()
        .map(|(idx, chart_cfg)| {
            render_metric_chart(
                chart_cfg,
                idx,
                name,
                active_window,
                &metrics_guard,
                start,
                end,
            )
        })
        .collect();
    drop(metrics_guard);

    html! {
        (time_selector)

        h2 { "> uptime" }
        div.(ChartClass::CHART_CONTAINER) {
            (uptime_section)
        }

        @if !chart_markup.is_empty() {
            h2 { "> metrics" }
            @for chart in &chart_markup {
                (chart)
            }
        }

        div.(ChartClass::CHART_ACTIONS) {
            button.(ChartClass::ADD_CHART_BTN)
                hx-get=(format!("/fragments/bot/{name}/add-chart/events?window={active_window}"))
                hx-target=(format!(".{}", ChartClass::CHART_ACTIONS))
                hx-swap="innerHTML"
            {
                "[+ add chart]"
            }
        }
    }
}

fn render_time_selector(name: &str, active_window: &str) -> Markup {
    html! {
        div.(ChartClass::TIME_WINDOW_SELECTOR) {
            @for &(key, label, _) in TIME_WINDOWS {
                button
                    .(ChartClass::TIME_WINDOW_BTN)
                    .(if key == active_window { ChartClass::TIME_WINDOW_ACTIVE } else { "" })
                    hx-get=(format!("/fragments/bot/{name}/charts?window={key}"))
                    hx-target="#charts-container"
                    hx-swap="innerHTML"
                {
                    (label)
                }
            }
        }
    }
}

fn render_uptime_section(
    state: &AppState,
    name: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Markup {
    let registry = state.registry.read().unwrap();
    if let Some(bot) = registry.get(name) {
        svg::render_uptime_chart(&bot.heartbeat_history, start, end, CHART_BUCKET_COUNT)
    } else {
        svg::render_uptime_chart(
            &std::collections::VecDeque::new(),
            start,
            end,
            CHART_BUCKET_COUNT,
        )
    }
}

fn render_metric_chart(
    chart_cfg: &ChartConfig,
    idx: usize,
    name: &str,
    active_window: &str,
    metrics: &crate::metrics::MetricStore,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Markup {
    let events = metrics.query_window(
        name,
        &chart_cfg.event_id,
        start,
        end,
        &chart_cfg.tag_filters,
    );
    let available_tags = metrics.available_tags(name, &chart_cfg.event_id);

    let chart_html = match chart_cfg.chart_type {
        ChartType::SingleValue => {
            let total: f64 = match events.len() {
                0 => 0.0,
                _ => events.iter().filter_map(|e| e.value).sum::<f64>(),
            };
            let count = events.len();
            let display = if metrics.has_values(name, &chart_cfg.event_id) {
                format!("{total}")
            } else {
                format!("{count}")
            };
            html! {
                div.(ChartClass::SINGLE_VALUE_DISPLAY) {
                    div.(ChartClass::SINGLE_VALUE_NUMBER) { (display) }
                    div.(ChartClass::SINGLE_VALUE_LABEL) { (chart_cfg.event_id) }
                }
            }
        }
        ref ct => {
            let bucketed =
                charts::bucket_events(&events, start, end, CHART_BUCKET_COUNT, MIN_BUCKET_SECONDS);
            let aggregated = match ct {
                ChartType::EventCountBar => charts::aggregate_count(&bucketed),
                ChartType::ValueSumBar => charts::aggregate_sum(&bucketed),
                ChartType::ValueAverageLine => charts::aggregate_average(&bucketed),
                ChartType::SingleValue => unreachable!(),
            };
            let label = format!("{} — {}", chart_cfg.event_id, ct.display_name());
            match ct {
                ChartType::EventCountBar | ChartType::ValueSumBar => {
                    svg::render_bar_chart(&aggregated, &label)
                }
                ChartType::ValueAverageLine => svg::render_line_chart(&aggregated, &label),
                ChartType::SingleValue => unreachable!(),
            }
        }
    };

    html! {
        div.(ChartClass::CHART_CONTAINER) {
            div.(ChartClass::CHART_HEADER) {
                span { (chart_cfg.event_id) " — " (chart_cfg.chart_type.display_name()) }
                button.(ChartClass::REMOVE_BTN)
                    hx-delete=(format!("/bot/{name}/charts/{idx}"))
                    hx-target="#charts-container"
                    hx-swap="innerHTML"
                    hx-confirm="Remove this chart?"
                { "[x]" }
            }
            // Tag filters
            @if !available_tags.is_empty() {
                div.(ChartClass::TAG_FILTER) {
                    @for (tag_key, tag_values) in &available_tags {
                        @let current = chart_cfg.tag_filters.get(tag_key.as_str());
                        label { (tag_key) ": " }
                        @for v in tag_values {
                            @let is_active = current.is_some_and(|cv| cv == v);
                            @if is_active {
                                button.(ChartClass::TAG_FILTER_BTN).(ChartClass::TAG_FILTER_ACTIVE)
                                    hx-get=(format!("/bot/{name}/charts/{idx}/filter?window={active_window}&tag_key={tag_key}&tag_value="))
                                    hx-target="#charts-container"
                                    hx-swap="innerHTML"
                                { (v) }
                            } @else {
                                button.(ChartClass::TAG_FILTER_BTN)
                                    hx-get=(format!("/bot/{name}/charts/{idx}/filter?window={active_window}&tag_key={tag_key}&tag_value={v}"))
                                    hx-target="#charts-container"
                                    hx-swap="innerHTML"
                                { (v) }
                            }
                        }
                    }
                }
            }
            (chart_html)
        }
    }
}
