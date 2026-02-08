use std::collections::{HashMap, HashSet, VecDeque};

use chrono::{DateTime, Duration, Utc};

pub struct MetricEvent {
    pub event_id: String,
    pub value: Option<f64>,
    #[allow(dead_code)]
    pub tags: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
}

pub struct MetricStore {
    metrics: HashMap<String, VecDeque<MetricEvent>>,
    retention: Duration,
}

impl MetricStore {
    pub fn new(retention: Duration) -> Self {
        MetricStore {
            metrics: HashMap::new(),
            retention,
        }
    }

    pub fn record(
        &mut self,
        bot_name: &str,
        event_id: String,
        value: Option<f64>,
        tags: HashMap<String, String>,
    ) -> DateTime<Utc> {
        let timestamp = Utc::now();
        let events = self.metrics.entry(bot_name.to_owned()).or_default();
        events.push_back(MetricEvent {
            event_id,
            value,
            tags,
            timestamp,
        });
        timestamp
    }

    pub fn query<'a>(
        &'a self,
        bot_name: &str,
        event_id_filter: Option<&str>,
    ) -> Vec<&'a MetricEvent> {
        let Some(events) = self.metrics.get(bot_name) else {
            return Vec::new();
        };
        match event_id_filter {
            Some(filter) => events.iter().filter(|e| e.event_id == filter).collect(),
            None => events.iter().collect(),
        }
    }

    pub fn event_ids(&self, bot_name: &str) -> Vec<String> {
        let Some(events) = self.metrics.get(bot_name) else {
            return Vec::new();
        };
        let mut ids: Vec<String> = events
            .iter()
            .map(|e| e.event_id.clone())
            .collect::<HashSet<_>>() // de-dupe
            .into_iter()
            .collect();
        ids.sort();
        ids
    }

    pub fn prune(&mut self) {
        let cutoff = Utc::now() - self.retention;
        for events in self.metrics.values_mut() {
            while events.front().is_some_and(|e| e.timestamp < cutoff) {
                events.pop_front();
            }
        }
        self.metrics.retain(|_, events| !events.is_empty());
    }

    pub fn remove_bot(&mut self, name: &str) {
        self.metrics.remove(name);
    }
}
