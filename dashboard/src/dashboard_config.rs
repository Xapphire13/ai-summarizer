use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::state::DASHBOARDS_DIR;

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

pub fn load(bot_name: &str) -> DashboardConfig {
    let path = Path::new(DASHBOARDS_DIR).join(format!("{bot_name}.toml"));
    match fs::read_to_string(&path) {
        Ok(content) => toml::from_str(&content).unwrap_or_default(),
        Err(_) => DashboardConfig::default(),
    }
}

pub fn save(bot_name: &str, config: &DashboardConfig) {
    let dir = Path::new(DASHBOARDS_DIR);
    let _ = fs::create_dir_all(dir);
    let path = dir.join(format!("{bot_name}.toml"));
    if let Ok(content) = toml::to_string_pretty(config) {
        let _ = fs::write(&path, content);
    }
}
