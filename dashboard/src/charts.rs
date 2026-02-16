pub mod svg;

use chrono::{DateTime, Utc};

use crate::metrics::MetricEvent;

pub fn bucket_events<'a>(
    events: &[&'a MetricEvent],
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    num_buckets: usize,
    min_bucket_secs: i64,
) -> Vec<(DateTime<Utc>, Vec<&'a MetricEvent>)> {
    let total_secs = (end - start).num_seconds();
    let bucket_secs = (total_secs / num_buckets as i64).max(min_bucket_secs);
    let actual_buckets = (total_secs / bucket_secs).max(1) as usize;

    let mut buckets: Vec<(DateTime<Utc>, Vec<&MetricEvent>)> = (0..actual_buckets)
        .map(|i| {
            let bucket_start = start + chrono::Duration::seconds(bucket_secs * i as i64);
            (bucket_start, Vec::new())
        })
        .collect();

    for event in events {
        let offset = (event.timestamp - start).num_seconds();
        if offset < 0 {
            continue;
        }
        let idx = (offset / bucket_secs) as usize;
        let idx = idx.min(actual_buckets - 1);
        buckets[idx].1.push(event);
    }

    buckets
}

pub fn aggregate_count(
    buckets: &[(DateTime<Utc>, Vec<&MetricEvent>)],
) -> Vec<(DateTime<Utc>, f64)> {
    buckets
        .iter()
        .map(|(ts, events)| (*ts, events.len() as f64))
        .collect()
}

pub fn aggregate_sum(buckets: &[(DateTime<Utc>, Vec<&MetricEvent>)]) -> Vec<(DateTime<Utc>, f64)> {
    buckets
        .iter()
        .map(|(ts, events)| {
            let sum: f64 = events.iter().filter_map(|e| e.value).sum();
            (*ts, sum)
        })
        .collect()
}

pub fn aggregate_average(
    buckets: &[(DateTime<Utc>, Vec<&MetricEvent>)],
) -> Vec<(DateTime<Utc>, f64)> {
    buckets
        .iter()
        .map(|(ts, events)| {
            let values: Vec<f64> = events.iter().filter_map(|e| e.value).collect();
            let avg = if values.is_empty() {
                0.0
            } else {
                values.iter().sum::<f64>() / values.len() as f64
            };
            (*ts, avg)
        })
        .collect()
}
