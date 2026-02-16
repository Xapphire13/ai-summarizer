use std::sync::Arc;

use crate::config::{DATA_RETENTION, PRUNE_INTERVAL};
use crate::state::AppState;

pub fn spawn_background_workers(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(PRUNE_INTERVAL);
        loop {
            interval.tick().await;

            // Two-phase prune:
            // 1. `prune_heartbeat_history` trims each bot's heartbeat deque to
            //    the retention window and removes bots whose history is now empty
            //    (i.e., all their heartbeats are older than the retention cutoff).
            // 2. `stale_bot_names` catches bots that still have recent-ish
            //    heartbeats in their deque but whose *last_heartbeat* timestamp
            //    falls outside the retention window — these are bots that stopped
            //    sending heartbeats but haven't aged out of the history yet.
            let stale_names = {
                let mut registry = state.registry.write().unwrap();
                registry.prune_heartbeat_history(DATA_RETENTION);

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
