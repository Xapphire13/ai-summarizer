use std::sync::Arc;

use rocket::{launch, routes};

use crate::background::BackgroundWorkers;
use crate::state::AppState;

mod background;
mod metrics;
mod registry;
mod routes;
mod state;
mod views;

#[launch]
fn rocket() -> _ {
    rocket::build()
        .manage(Arc::new(AppState::new()))
        .attach(BackgroundWorkers)
        .mount(
            "/",
            routes![
                routes::heartbeat,
                routes::record_metric,
                views::index,
                views::bot_detail,
                views::fragment_bot_list,
                views::fragment_bot_charts,
            ],
        )
}
