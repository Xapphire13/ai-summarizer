use std::sync::Arc;

use crate::state::{AppState, DATA_RETENTION, PRUNE_INTERVAL};

pub fn spawn_background_workers(state: Arc<AppState>) {
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
