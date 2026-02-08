use std::sync::RwLock;

use chrono::Duration;

use crate::metrics::MetricStore;
use crate::registry::BotRegistry;

pub const DATA_RETENTION: Duration = Duration::days(7);
pub const ONLINE_GRACE_PERIOD: Duration = Duration::minutes(5);
pub const PRUNE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(3600);

pub struct AppState {
    pub registry: RwLock<BotRegistry>,
    pub metrics: RwLock<MetricStore>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            registry: RwLock::new(BotRegistry::new()),
            metrics: RwLock::new(MetricStore::new(DATA_RETENTION)),
        }
    }
}
