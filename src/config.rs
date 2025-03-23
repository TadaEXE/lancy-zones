use serde::{Deserialize, Serialize};

use crate::util::Monitor;

#[derive(Debug, Serialize, Deserialize)]
pub struct MonitorConfig {
    pub name: String,
    pub zones: Vec<Zone>,
    pub monitor: Monitor,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Zone {
    pub id: u16,
    pub x: i16,
    pub y: i16,
    pub width: i16,
    pub height: i16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub monitor_configs: Vec<MonitorConfig>,
}
