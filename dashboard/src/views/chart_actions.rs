use std::collections::HashMap;
use std::sync::Arc;

use maud::{Markup, html};
use rocket::serde::json::Json;
use rocket::{State, delete, get, post};
use serde::Deserialize;

use crate::dashboard_config::{self, ChartConfig, ChartType};
use crate::state::AppState;
use crate::styles::Charts as ChartClass;

use super::bot_detail::fragment_bot_charts;

#[get("/fragments/bot/<name>/add-chart/events?<window>")]
pub fn add_chart_events(name: &str, window: Option<&str>, state: &State<Arc<AppState>>) -> Markup {
    let metrics = state.metrics.read().unwrap();
    let event_ids = metrics.event_ids(name);
    drop(metrics);

    let w = window.unwrap_or("1d");

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

#[get("/fragments/bot/<name>/add-chart/types?<event_id>&<window>")]
pub fn add_chart_types(
    name: &str,
    event_id: &str,
    window: Option<&str>,
    state: &State<Arc<AppState>>,
) -> Markup {
    let metrics = state.metrics.read().unwrap();
    let has_values = metrics.has_values(name, event_id);
    drop(metrics);

    let types = if has_values {
        ChartType::valid_for_valued()
    } else {
        ChartType::valid_for_valueless()
    };

    let w = window.unwrap_or("1d");

    html! {
        h3 { "> select chart type for " (event_id) }
        @for ct in &types {
            button.(ChartClass::ADD_CHART_BTN)
                hx-post=(format!("/bot/{name}/charts?window={w}"))
                hx-target="#charts-container"
                hx-swap="innerHTML"
                hx-vals=(serde_json::to_string(&serde_json::json!({
                    "event_id": event_id,
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

#[post("/bot/<name>/charts?<window>", format = "json", data = "<data>")]
pub fn add_chart(
    name: &str,
    window: Option<&str>,
    data: Json<AddChartRequest>,
    state: &State<Arc<AppState>>,
) -> Option<Markup> {
    let mut config = dashboard_config::load(name);
    config.charts.push(ChartConfig {
        event_id: data.event_id.clone(),
        chart_type: data.chart_type.clone(),
        tag_filters: HashMap::new(),
    });
    dashboard_config::save(name, &config);
    fragment_bot_charts(name, window, state)
}

#[delete("/bot/<name>/charts/<index>?<window>")]
pub fn remove_chart(
    name: &str,
    index: usize,
    window: Option<&str>,
    state: &State<Arc<AppState>>,
) -> Option<Markup> {
    let mut config = dashboard_config::load(name);
    if index < config.charts.len() {
        config.charts.remove(index);
        dashboard_config::save(name, &config);
    }
    fragment_bot_charts(name, window, state)
}

#[get("/bot/<name>/charts/<index>/filter?<window>&<tag_key>&<tag_value>")]
pub fn update_chart_filter(
    name: &str,
    index: usize,
    window: Option<&str>,
    tag_key: &str,
    tag_value: &str,
    state: &State<Arc<AppState>>,
) -> Option<Markup> {
    let mut config = dashboard_config::load(name);
    if let Some(chart) = config.charts.get_mut(index) {
        if tag_value.is_empty() {
            chart.tag_filters.remove(tag_key);
        } else {
            chart
                .tag_filters
                .insert(tag_key.to_owned(), tag_value.to_owned());
        }
        dashboard_config::save(name, &config);
    }
    fragment_bot_charts(name, window, state)
}
