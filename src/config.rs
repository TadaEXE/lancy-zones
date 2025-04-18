use std::{
    fs::{self, File, read},
    io::Write,
    path::Path,
};

use serde::{Deserialize, Serialize};
use x11rb::connection::Connection;

use crate::util::{self, Monitor};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub monitor_configs: Vec<MonitorConfig>,
}

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

impl Zone {
    pub fn get_center_point(&self) -> (i16, i16) {
        (
            ((self.x + self.width) / 2) as i16,
            ((self.y + self.height) / 2) as i16,
        )
    }
}

pub fn init_cfg_file<C: Connection>(path: &Path, conn: &C, root: u32) {
    let monitors = util::get_monitors(conn, root).unwrap();
    let mut monitor_configs = vec![];

    for monitor in &monitors {
        let zones = vec![Zone {
            id: 0,
            x: 0,
            y: 0,
            width: monitor.width.try_into().unwrap(),
            height: monitor.height.try_into().unwrap(),
        }];
        monitor_configs.push(MonitorConfig {
            name: monitor.name.clone(),
            zones,
            monitor: monitor.clone(),
        });
    }

    let config = Config { monitor_configs };
    let dir = path.parent().unwrap_or(Path::new("/"));
    fs::create_dir_all(&dir).expect(&format!("Failed to create dir at {:#?}", dir));
    let mut cfg_file = File::create(path).expect(&format!(
        "Failed to create fresh config file at {:#?}.",
        path
    ));
    let data = serde_json::to_vec(&config).unwrap();
    _ = cfg_file
        .write(&data)
        .expect(&format!("Failed to write config file at {:#?}", path));
}

pub fn load_cfg_file(path: &Path) -> Config {
    let data = read(path).expect(&format!("Failed to read config file at {:#?}", path));
    serde_json::from_slice(&data).unwrap()
}

pub fn save_cfg_file(path: &Path, config: &Config) {
    let data = serde_json::to_vec(config).unwrap();
    let mut file = File::open(path).expect(&format!("Failed to open config file at {:#?}", path));
    _ = file
        .write(&data)
        .expect(&format!("Failed to write config file at {:#?}", path));
}
