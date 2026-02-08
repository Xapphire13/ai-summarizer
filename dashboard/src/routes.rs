use std::{collections::HashMap, sync::Arc};

use rocket::{State, post, serde::json::Json};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Deserialize)]
pub struct HeartbeatRequest {
    name: String,
}

#[derive(Serialize)]
pub struct HeartbeatResponse {
    name: String,
    registered_at: String,
    last_heartbeat: String,
}

#[post("/heartbeat", format = "json", data = "<data>")]
pub fn heartbeat(
    data: Json<HeartbeatRequest>,
    state: &State<Arc<AppState>>,
) -> Json<HeartbeatResponse> {
    let mut registry = state.registry.write().unwrap();
    let info = registry.log_heartbeat(&data.name);

    Json(HeartbeatResponse {
        name: info.name.clone(),
        registered_at: info.registered_at.to_rfc3339(),
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
}

#[derive(Serialize)]
pub struct MetricResponse {
    status: String,
    timestamp: String,
}

#[post("/metrics", format = "json", data = "<data>")]
pub fn record_metric(
    data: Json<MetricRequest>,
    state: &State<Arc<AppState>>,
) -> (rocket::http::Status, Json<MetricResponse>) {
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
        )
    };

    (
        rocket::http::Status::Created,
        Json(MetricResponse {
            status: "recorded".to_owned(),
            timestamp: timestamp.to_rfc3339(),
        }),
    )
}
