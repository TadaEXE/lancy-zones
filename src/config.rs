use core::fmt;
use std::{
    fs::{self, File, read},
    io::Write,
    ops::DerefMut,
    path::Path,
};

use serde::{Deserialize, Serialize};
use x11rb::connection::Connection;

use crate::util::{self, Monitor};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub monitor_configs: Vec<MonitorConfig>,
    pub alpha: f32,
    pub line_thickness: u16,
}

impl Config {
    pub fn get_monitor_config(&self, mc_name: &str) -> Option<&MonitorConfig> {
        self.monitor_configs
            .iter()
            .find(|cfg| -> bool { cfg.name == mc_name })
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:#?} [alpha: {}; line_thickness: {}]",
            self.monitor_configs, self.alpha, self.line_thickness
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MonitorConfig {
    pub name: String,
    pub zones: Vec<Zone>,
    pub monitor: Monitor,
    pub active: bool,
}

impl fmt::Display for MonitorConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: [{:#?}] ({})", self.name, self.zones, self.monitor)
    }

    pub fn remove_zone(&mut self, mc_name: &str, zone_name: &str) {
        if let Some(index) = self
            .zones
            .iter()
            .position(|zone| -> bool { zone.id == zone_id })
        {
            monitor_cfg.zones.remove(index);
        } else {
            println!(
                "Could not find Zone with id {} assigned to config {}",
                zone_name, mc_name
            )
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Zone {
    pub name: String,
    pub x: i16,
    pub y: i16,
    pub width: i16,
    pub height: i16,
}

impl MonitorConfig {
    pub fn add_zone(&mut self, mc_name: &str, zone: Zone) {
        self.zones.push(zone);
    }
}

impl fmt::Display for Zone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} at x: {} y: {} => {}x{}",
            self.name, self.x, self.y, self.width, self.height
        )
    }
}

impl Zone {
    pub fn get_center_point(&self) -> (i16, i16) {
        (
            (self.width / 2 + self.x) as i16,
            (self.height / 2 + self.y) as i16,
        )
    }

    pub fn is_inside(&self, x: i16, y: i16) -> bool {
        true
    }

    pub fn get_sqr_dist_to(&self, x: i16, y: i16) -> u32 {
        let cp = self.get_center_point();
        (((x - cp.0) as i32).pow(2) + ((y - cp.1) as i32).pow(2))
            .try_into()
            .expect("Failed to calculate square distance")
    }
}

pub fn init_cfg_file<C: Connection>(path: &Path, conn: &C, root: u32) {
    let monitors = util::get_monitors(conn, root).unwrap();
    let mut monitor_configs = vec![];

    for monitor in &monitors {
        let zones = vec![Zone {
            name: "unnamed".to_string(),
            x: 0,
            y: 0,
            width: monitor.width.try_into().unwrap(),
            height: monitor.height.try_into().unwrap(),
        }];
        monitor_configs.push(MonitorConfig {
            name: monitor.name.clone(),
            zones,
            monitor: monitor.clone(),
            active: true,
        });
    }

    let config = Config {
        monitor_configs,
        alpha: 0.5,
        line_thickness: 3,
    };
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
