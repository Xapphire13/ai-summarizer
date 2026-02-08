use std::sync::Arc;

use rocket::Rocket;
use rocket::fairing::{Fairing, Info, Kind};

use crate::state::{AppState, DATA_RETENTION, PRUNE_INTERVAL};

pub struct BackgroundWorkers;

#[rocket::async_trait]
impl Fairing for BackgroundWorkers {
    fn info(&self) -> Info {
        Info {
            name: "Background Workers",
            kind: Kind::Liftoff,
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<rocket::Orbit>) {
        let state = rocket
            .state::<Arc<AppState>>()
            .expect("AppState must be managed");

        let state = Arc::clone(state);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(PRUNE_INTERVAL);
            loop {
                interval.tick().await;

                // Prune old data
                let stale_names = {
                    let mut registry = state.registry.write().unwrap();
                    registry.prune_heartbeat_history(DATA_RETENTION);

                    // Remove stale bots
                    let stale_names = registry.stale_bot_names(DATA_RETENTION);
                    for name in &stale_names {
                        registry.remove(name);
                    }

                    stale_names
                };

                {
                    let mut metrics = state.metrics.write().unwrap();
                    metrics.prune();
                    for name in &stale_names {
                        metrics.remove_bot(name);
                    }
                }
            }
        });
    }
}
