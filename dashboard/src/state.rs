use std::path::PathBuf;
use std::sync::RwLock;

use chrono::Duration;

use crate::metrics::MetricStore;
use crate::registry::BotRegistry;

pub const DATA_RETENTION: Duration = Duration::days(7);
pub const ONLINE_GRACE_PERIOD: Duration = Duration::minutes(5);
pub const PRUNE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(3600);

pub const CHART_BUCKET_COUNT: usize = 100;
pub const MIN_BUCKET_SECONDS: i64 = 1;
pub const DASHBOARDS_DIR: &str = "data/dashboards";

const METRICS_DIR: &str = "data/metrics";
const HEARTBEATS_DIR: &str = "data/heartbeats";

pub struct AppState {
    pub registry: RwLock<BotRegistry>,
    pub metrics: RwLock<MetricStore>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            registry: RwLock::new(BotRegistry::new(PathBuf::from(HEARTBEATS_DIR))),
            metrics: RwLock::new(MetricStore::new(DATA_RETENTION, PathBuf::from(METRICS_DIR))),
        }
    }
}
