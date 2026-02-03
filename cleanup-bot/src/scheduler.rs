use std::sync::{Arc, Mutex};
use std::time::Duration;

use serenity::all::Http;
use tokio::time::{MissedTickBehavior, interval};
use tracing::{debug, info};

use crate::cancellation::CancellationRegistry;
use crate::cleanup::cleanup_channel;
use crate::config::ConfigStore;

/// Spawn the cleanup scheduler task.
pub fn spawn_scheduler(
    http: Arc<Http>,
    config: ConfigStore,
    cancellation: Arc<Mutex<CancellationRegistry>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        run_scheduler(http, config, cancellation).await;
    })
}

async fn run_scheduler(
    http: Arc<Http>,
    config: ConfigStore,
    cancellation: Arc<Mutex<CancellationRegistry>>,
) {
    let scheduler_interval = Duration::from_secs(config.schedule_interval_seconds().get() as u64);
    let mut interval = interval(scheduler_interval);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    info!(
        "Cleanup scheduler started (interval: {:?})",
        scheduler_interval
    );

    loop {
        interval.tick().await;

        // Get enabled channels snapshot
        let channels = config.enabled_channels();

        if channels.is_empty() {
            debug!("No enabled channels, skipping cleanup tick");
            continue;
        }

        info!(
            "Scheduler tick: processing {} enabled channel(s)",
            channels.len()
        );

        // Spawn independent cleanup tasks for each channel
        for (channel_id, retention_days) in channels {
            let http = Arc::clone(&http);
            let config = config.clone();
            let cancellation_registry = Arc::clone(&cancellation);

            // Check and register atomically to prevent race condition
            let cancel_token = {
                let mut registry = cancellation_registry.lock().unwrap();
                if registry.is_running(channel_id) {
                    debug!(
                        "Cleanup already running for channel {}, skipping",
                        channel_id
                    );
                    continue;
                }
                registry.register(channel_id)
            };

            debug!(
                "Spawning cleanup task for channel {} (retention: {} days)",
                channel_id, retention_days
            );

            tokio::spawn(async move {
                cleanup_channel(
                    http,
                    config,
                    cancellation_registry,
                    channel_id,
                    retention_days,
                    cancel_token,
                )
                .await;
            });
        }
    }
}
