//! Behavioral constants for data retention, pruning, and chart rendering.

use chrono::Duration;

pub const DATA_RETENTION: Duration = Duration::days(7);
pub const ONLINE_GRACE_PERIOD: Duration = Duration::minutes(5);
pub const PRUNE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(3600);

pub const CHART_BUCKET_COUNT: usize = 100;
pub const MIN_BUCKET_SECONDS: i64 = 1;
