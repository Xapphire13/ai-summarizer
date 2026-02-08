use std::collections::{HashMap, VecDeque};

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct BotInfo {
    pub name: String,
    pub registered_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    #[serde(skip)]
    pub heartbeat_history: VecDeque<DateTime<Utc>>,
}

pub struct BotRegistry {
    bots: HashMap<String, BotInfo>,
}

impl BotRegistry {
    pub fn new() -> Self {
        BotRegistry {
            bots: HashMap::new(),
        }
    }

    pub fn log_heartbeat(&mut self, name: &str) -> &BotInfo {
        let now = Utc::now();
        let info = self.bots.entry(name.to_owned()).or_insert_with(|| BotInfo {
            name: name.to_owned(),
            registered_at: now,
            last_heartbeat: now,
            heartbeat_history: VecDeque::new(),
        });
        info.last_heartbeat = now;
        info.heartbeat_history.push_back(now);
        info
    }

    pub fn ensure_registered(&mut self, name: &str) -> &BotInfo {
        let now = Utc::now();
        self.bots.entry(name.to_owned()).or_insert_with(|| BotInfo {
            name: name.to_owned(),
            registered_at: now,
            last_heartbeat: now,
            heartbeat_history: VecDeque::new(),
        })
    }

    pub fn bots(&self) -> Vec<&BotInfo> {
        self.bots.values().collect()
    }

    pub fn get(&self, name: &str) -> Option<&BotInfo> {
        self.bots.get(name)
    }

    pub fn remove(&mut self, name: &str) {
        self.bots.remove(name);
    }

    pub fn stale_bot_names(&self, max_age: Duration) -> Vec<String> {
        let cutoff = Utc::now() - max_age;
        self.bots
            .iter()
            .filter(|(_, info)| info.last_heartbeat < cutoff)
            .map(|(name, _)| name.clone())
            .collect()
    }

    pub fn is_online(&self, name: &str, grace_period: Duration) -> bool {
        self.bots
            .get(name)
            .is_some_and(|info| Utc::now() - info.last_heartbeat < grace_period)
    }

    pub fn prune_heartbeat_history(&mut self, retention: Duration) {
        let cutoff = Utc::now() - retention;
        for info in self.bots.values_mut() {
            while info
                .heartbeat_history
                .front()
                .is_some_and(|ts| *ts < cutoff)
            {
                info.heartbeat_history.pop_front();
            }
        }
    }
}
