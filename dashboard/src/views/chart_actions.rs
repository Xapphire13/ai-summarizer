use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Json, Path, Query, State};
use axum::http::StatusCode;
use maud::{Markup, html};
use serde::Deserialize;

use crate::dashboard_config::{self, ChartConfig, ChartType};
use crate::state::AppState;
use crate::styles::Charts as ChartClass;

use super::WindowQuery;
use super::bot_detail;

#[derive(Deserialize)]
pub struct AddChartTypesQuery {
    event_id: String,
    window: Option<String>,
}

#[derive(Deserialize)]
pub struct FilterQuery {
    window: Option<String>,
    tag_key: String,
    tag_value: String,
}

pub async fn add_chart_events(
    Path(name): Path<String>,
    Query(query): Query<WindowQuery>,
    State(state): State<Arc<AppState>>,
) -> Markup {
    let metrics = state.metrics.read().unwrap();
    let event_ids = metrics.event_ids(&name);
    drop(metrics);

    let w = query.window.as_deref().unwrap_or("1d");

    html! {
        h3 { "> select event" }
        @if event_ids.is_empty() {
            p { "No events recorded yet." }
        } @else {
            @for eid in &event_ids {
                button.(ChartClass::ADD_CHART_BTN)
                    hx-get=(format!("/fragments/bot/{name}/add-chart/types?event_id={eid}&window={w}"))
                    hx-target=(format!(".{}", ChartClass::CHART_ACTIONS))
                    hx-swap="innerHTML"
                {
                    (eid)
                }
            }
        }
        button.(ChartClass::ADD_CHART_BTN)
            hx-get=(format!("/fragments/bot/{name}/charts?window={w}"))
            hx-target="#charts-container"
            hx-swap="innerHTML"
        {
            "[cancel]"
        }
    }
}

pub async fn add_chart_types(
    Path(name): Path<String>,
    Query(query): Query<AddChartTypesQuery>,
    State(state): State<Arc<AppState>>,
) -> Markup {
    let metrics = state.metrics.read().unwrap();
    let has_values = metrics.has_values(&name, &query.event_id);
    drop(metrics);

    let types = if has_values {
        ChartType::valid_for_valued()
    } else {
        ChartType::valid_for_valueless()
    };

    let w = query.window.as_deref().unwrap_or("1d");

    html! {
        h3 { "> select chart type for " (query.event_id) }
        @for ct in &types {
            button.(ChartClass::ADD_CHART_BTN)
                hx-post=(format!("/bot/{name}/charts?window={w}"))
                hx-target="#charts-container"
                hx-swap="innerHTML"
                hx-vals=(serde_json::to_string(&serde_json::json!({
                    "event_id": query.event_id,
                    "chart_type": ct,
                })).unwrap())
            {
                (ct.display_name())
            }
        }
        button.(ChartClass::ADD_CHART_BTN)
            hx-get=(format!("/fragments/bot/{name}/charts?window={w}"))
            hx-target="#charts-container"
            hx-swap="innerHTML"
        {
            "[cancel]"
        }
    }
}

#[derive(Deserialize)]
pub struct AddChartRequest {
    event_id: String,
    chart_type: ChartType,
}

pub async fn add_chart(
    Path(name): Path<String>,
    Query(query): Query<WindowQuery>,
    State(state): State<Arc<AppState>>,
    Json(data): Json<AddChartRequest>,
) -> Result<Markup, StatusCode> {
    let mut config = dashboard_config::load(&name).map_err(|e| {
        eprintln!("warning: failed to load dashboard config for {name}: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    config.charts.push(ChartConfig {
        event_id: data.event_id.clone(),
        chart_type: data.chart_type.clone(),
        tag_filters: HashMap::new(),
    });
    if let Err(e) = dashboard_config::save(&name, &config) {
        eprintln!("warning: failed to save dashboard config for {name}: {e}");
    }
    Ok(bot_detail::render_charts(
        &name,
        query.window.as_deref(),
        &state,
    ))
}

pub async fn remove_chart(
    Path((name, index)): Path<(String, usize)>,
    Query(query): Query<WindowQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<Markup, StatusCode> {
    let mut config = dashboard_config::load(&name).map_err(|e| {
        eprintln!("warning: failed to load dashboard config for {name}: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    if index < config.charts.len() {
        config.charts.remove(index);
        if let Err(e) = dashboard_config::save(&name, &config) {
            eprintln!("warning: failed to save dashboard config for {name}: {e}");
        }
    }
    Ok(bot_detail::render_charts(
        &name,
        query.window.as_deref(),
        &state,
    ))
}

pub async fn update_chart_filter(
    Path((name, index)): Path<(String, usize)>,
    Query(query): Query<FilterQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<Markup, StatusCode> {
    let mut config = dashboard_config::load(&name).map_err(|e| {
        eprintln!("warning: failed to load dashboard config for {name}: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    if let Some(chart) = config.charts.get_mut(index) {
        if query.tag_value.is_empty() {
            chart.tag_filters.remove(&query.tag_key);
        } else {
            chart
                .tag_filters
                .insert(query.tag_key.to_owned(), query.tag_value.to_owned());
        }
        if let Err(e) = dashboard_config::save(&name, &config) {
            eprintln!("warning: failed to save dashboard config for {name}: {e}");
        }
    }
    Ok(bot_detail::render_charts(
        &name,
        query.window.as_deref(),
        &state,
    ))
}
