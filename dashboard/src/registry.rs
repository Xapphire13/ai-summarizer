use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::storage;

#[derive(Clone, Serialize)]
pub struct BotInfo {
    pub name: String,
    pub last_heartbeat: DateTime<Utc>,
    #[serde(skip)]
    pub heartbeat_history: VecDeque<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize)]
struct HeartbeatRecord {
    timestamp: DateTime<Utc>,
}

pub struct BotRegistry {
    bots: HashMap<String, BotInfo>,
    data_dir: PathBuf,
}

impl BotRegistry {
    pub fn new(data_dir: PathBuf) -> Self {
        let mut bots: HashMap<String, BotInfo> = HashMap::new();

        if let Ok(bot_names) = storage::discover_bots(&data_dir) {
            for bot_name in bot_names {
                match storage::load_lines::<HeartbeatRecord>(&data_dir, &bot_name) {
                    Ok(records) => {
                        if records.is_empty() {
                            continue;
                        }
                        let history: VecDeque<DateTime<Utc>> =
                            records.iter().map(|r| r.timestamp).collect();
                        let last_heartbeat = history.back().copied().unwrap_or_else(Utc::now);
                        bots.insert(
                            bot_name.clone(),
                            BotInfo {
                                name: bot_name,
                                last_heartbeat,
                                heartbeat_history: history,
                            },
                        );
                    }
                    Err(e) => {
                        eprintln!("warning: failed to load heartbeats for {bot_name}: {e}")
                    }
                }
            }
        }

        BotRegistry { bots, data_dir }
    }

    pub fn log_heartbeat(&mut self, name: &str) -> &BotInfo {
        let now = Utc::now();
        let info = self.bots.entry(name.to_owned()).or_insert_with(|| BotInfo {
            name: name.to_owned(),
            last_heartbeat: now,
            heartbeat_history: VecDeque::new(),
        });
        info.last_heartbeat = now;
        info.heartbeat_history.push_back(now);

        let record = HeartbeatRecord { timestamp: now };
        if let Err(e) = storage::append_line(&self.data_dir, name, &record) {
            eprintln!("warning: failed to persist heartbeat for {name}: {e}");
        }

        info
    }

    /// Ensures a bot exists in the registry without recording a heartbeat.
    ///
    /// Used by the metrics endpoint so that bots sending only metrics (no
    /// heartbeats) still appear in the bot list. Unlike `log_heartbeat`, this
    /// does not append to heartbeat history or persist to disk.
    pub fn ensure_registered(&mut self, name: &str) -> &BotInfo {
        let now = Utc::now();
        self.bots.entry(name.to_owned()).or_insert_with(|| BotInfo {
            name: name.to_owned(),
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
        if let Err(e) = storage::remove_bot_file(&self.data_dir, name) {
            eprintln!("warning: failed to remove heartbeat file for {name}: {e}");
        }
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

    /// Removes heartbeat entries older than `retention` from every bot's history.
    ///
    /// Bots whose history becomes empty are removed from the registry and their
    /// persisted `.jsonl` files are deleted. Surviving bots have their files
    /// rewritten to reflect the trimmed history (only if entries were actually
    /// removed).
    pub fn prune_heartbeat_history(&mut self, retention: Duration) {
        let cutoff = Utc::now() - retention;
        let mut dirty = Vec::new();

        for (name, info) in self.bots.iter_mut() {
            let before = info.heartbeat_history.len();
            while info
                .heartbeat_history
                .front()
                .is_some_and(|ts| *ts < cutoff)
            {
                info.heartbeat_history.pop_front();
            }
            if info.heartbeat_history.len() != before {
                dirty.push(name.clone());
            }
        }

        let empty_bots: Vec<String> = self
            .bots
            .iter()
            .filter(|(_, info)| info.heartbeat_history.is_empty())
            .map(|(name, _)| name.clone())
            .collect();
        for name in &empty_bots {
            self.bots.remove(name);
            if let Err(e) = storage::remove_bot_file(&self.data_dir, name) {
                eprintln!("warning: failed to remove heartbeat file for {name}: {e}");
            }
        }

        for name in &dirty {
            let Some(info) = self.bots.get(name) else {
                continue; // was in empty_bots
            };
            let records: Vec<HeartbeatRecord> = info
                .heartbeat_history
                .iter()
                .map(|&ts| HeartbeatRecord { timestamp: ts })
                .collect();
            if let Err(e) = storage::rewrite_lines(&self.data_dir, name, records.iter()) {
                eprintln!("warning: failed to rewrite heartbeats for {name}: {e}");
            }
        }
    }
}
