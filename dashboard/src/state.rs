use std::path::PathBuf;
use std::sync::RwLock;

use crate::config::DATA_RETENTION;
use crate::metrics::MetricStore;
use crate::paths::{HEARTBEATS_DIR, METRICS_DIR};
use crate::registry::BotRegistry;

/// Shared application state holding bot registration data and metric storage.
///
/// Both fields are wrapped in `RwLock` for concurrent access from request handlers
/// and background workers. Lock ordering discipline: always acquire `registry`
/// before `metrics` when both locks are needed simultaneously.
///
/// The `.unwrap()` calls on lock acquisition are intentional: if a thread panics
/// while holding a lock, the lock becomes poisoned. At that point the process is
/// in an unrecoverable state and should crash rather than silently continue with
/// potentially corrupted data.
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
