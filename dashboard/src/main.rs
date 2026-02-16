use std::sync::Arc;

use axum::Router;
use axum::routing::{delete, get, post};

use crate::state::AppState;

mod background;
mod charts;
mod dashboard_config;
mod metrics;
mod registry;
mod routes;
mod state;
mod storage;
mod styles;
mod views;

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState::new());

    background::spawn_background_workers(Arc::clone(&state));

    let app = Router::new()
        .route("/heartbeat", post(routes::heartbeat))
        .route("/metrics", post(routes::record_metric))
        .route("/styles.css", get(views::styles))
        .route("/", get(views::index))
        .route("/bot/{name}", get(views::bot_detail::bot_detail))
        .route(
            "/fragments/bot-list",
            get(views::bot_list::fragment_bot_list),
        )
        .route(
            "/fragments/bot/{name}/charts",
            get(views::bot_detail::fragment_bot_charts),
        )
        .route(
            "/fragments/bot/{name}/add-chart/events",
            get(views::chart_actions::add_chart_events),
        )
        .route(
            "/fragments/bot/{name}/add-chart/types",
            get(views::chart_actions::add_chart_types),
        )
        .route("/bot/{name}/charts", post(views::chart_actions::add_chart))
        .route(
            "/bot/{name}/charts/{index}",
            delete(views::chart_actions::remove_chart),
        )
        .route(
            "/bot/{name}/charts/{index}/filter",
            get(views::chart_actions::update_chart_filter),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000")
        .await
        .expect("failed to bind to port 8000");
    axum::serve(listener, app).await.unwrap();
}
