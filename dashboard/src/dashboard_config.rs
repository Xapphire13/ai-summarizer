use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::paths::DASHBOARDS_DIR;
use crate::storage;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct DashboardConfig {
    #[serde(default)]
    pub charts: Vec<ChartConfig>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChartConfig {
    pub event_id: String,
    pub chart_type: ChartType,
    #[serde(default)]
    pub tag_filters: HashMap<String, String>,
}

/// Chart visualization types.
///
/// `EventCountBar` and `SingleValue` work with any events (including valueless).
/// `ValueSumBar` and `ValueAverageLine` require events that carry numeric values.
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum ChartType {
    EventCountBar,
    ValueSumBar,
    ValueAverageLine,
    SingleValue,
}

impl ChartType {
    pub fn valid_for_valueless() -> Vec<ChartType> {
        vec![ChartType::EventCountBar, ChartType::SingleValue]
    }

    pub fn valid_for_valued() -> Vec<ChartType> {
        vec![
            ChartType::EventCountBar,
            ChartType::ValueSumBar,
            ChartType::ValueAverageLine,
            ChartType::SingleValue,
        ]
    }

    pub fn display_name(&self) -> &str {
        match self {
            ChartType::EventCountBar => "Event Count (Bar)",
            ChartType::ValueSumBar => "Value Sum (Bar)",
            ChartType::ValueAverageLine => "Value Average (Line)",
            ChartType::SingleValue => "Single Value",
        }
    }
}

/// Loads the dashboard config for a bot. Returns `DashboardConfig::default()` if
/// the file doesn't exist; propagates other I/O and parse errors.
pub fn load(bot_name: &str) -> io::Result<DashboardConfig> {
    let safe_name = storage::sanitize_bot_name(bot_name);
    let path = Path::new(DASHBOARDS_DIR).join(format!("{safe_name}.toml"));
    match fs::read_to_string(&path) {
        Ok(content) => {
            toml::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(DashboardConfig::default()),
        Err(e) => Err(e),
    }
}

/// Persists a dashboard config to disk for the given bot.
pub fn save(bot_name: &str, config: &DashboardConfig) -> io::Result<()> {
    let safe_name = storage::sanitize_bot_name(bot_name);
    let dir = Path::new(DASHBOARDS_DIR);
    fs::create_dir_all(dir)?;
    let path = dir.join(format!("{safe_name}.toml"));
    let content = toml::to_string_pretty(config)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(&path, content)
}
