use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MonitorConfig {
    pub name: String,
    pub zones: Vec<Zone>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Zone {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub monitors: Vec<MonitorConfig>,
}
