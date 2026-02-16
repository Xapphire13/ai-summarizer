use std::{collections::HashMap, sync::Arc};

use axum::extract::{Json, State};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Deserialize)]
pub struct HeartbeatRequest {
    name: String,
}

#[derive(Serialize)]
pub struct HeartbeatResponse {
    name: String,
    last_heartbeat: String,
}

pub async fn heartbeat(
    State(state): State<Arc<AppState>>,
    Json(data): Json<HeartbeatRequest>,
) -> Json<HeartbeatResponse> {
    let mut registry = state.registry.write().unwrap();
    let info = registry.log_heartbeat(&data.name);

    Json(HeartbeatResponse {
        name: info.name.clone(),
        last_heartbeat: info.last_heartbeat.to_rfc3339(),
    })
}

#[derive(Deserialize)]
pub struct MetricRequest {
    bot_name: String,
    event_id: String,
    value: Option<f64>,
    #[serde(default)]
    tags: HashMap<String, String>,
    #[serde(default)]
    timestamp: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct MetricResponse {
    status: String,
    timestamp: String,
}

pub async fn record_metric(
    State(state): State<Arc<AppState>>,
    Json(data): Json<MetricRequest>,
) -> (StatusCode, Json<MetricResponse>) {
    // Lock ordering: registry first, then metrics
    {
        let mut registry = state.registry.write().unwrap();
        registry.ensure_registered(&data.bot_name);
    }
    let timestamp = {
        let mut metrics = state.metrics.write().unwrap();
        metrics.record(
            &data.bot_name,
            data.event_id.clone(),
            data.value,
            data.tags.clone(),
            data.timestamp,
        )
    };

    (
        StatusCode::CREATED,
        Json(MetricResponse {
            status: "recorded".to_owned(),
            timestamp: timestamp.to_rfc3339(),
        }),
    )
}
