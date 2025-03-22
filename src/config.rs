use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MonitorConfig {
    pub name: String,
    pub zones: Vec<Zone>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Zone {
    pub x: i16,
    pub y: i16,
    pub width: i16,
    pub height: i16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub monitors: Vec<MonitorConfig>,
}
