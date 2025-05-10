use core::fmt;
use std::{
    fs::{self, File, read},
    io::Write,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use x11rb::connection::Connection;

use crate::util;

pub fn get_config_path() -> PathBuf {
    Path::new("~/.config/lancy-zones/config.json").to_path_buf()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub monitors: Vec<Monitor>,
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

    pub fn get_monitor_config_mut(&mut self, mc_name: &str) -> Option<&mut MonitorConfig> {
        self.monitor_configs
            .iter_mut()
            .find(|cfg| -> bool { cfg.name == mc_name })
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "alpha: {}\nline_thickness: {}\nmonitors: {:#?}\nconfigs: {:#?}",
            self.alpha, self.line_thickness, self.monitors, self.monitor_configs
        )
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Monitor {
    pub name: String,
    pub config: Option<MonitorConfig>,
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

impl fmt::Display for Monitor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({}x{}) cfg: {:#?} x: {} y: {}",
            self.name, self.width, self.height, self.config, self.x, self.y
        )
    }
}

impl Monitor {
    pub fn coords_inside(&self, x: i16, y: i16) -> bool {
        let val = x >= self.x
            && x <= self.x + self.width as i16
            && y >= self.y
            && y <= self.y + self.height as i16;
        val
    }

    pub fn to_local_space(&self, x: i16, y: i16) -> (i16, i16) {
        (x - self.x, y - self.y)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonitorConfig {
    pub name: String,
    pub zones: Vec<Zone>,
}

impl MonitorConfig {
    pub fn add_zone(&mut self, zone: Zone) {
        self.zones.push(zone);
    }

    pub fn remove_zone(&mut self, zone_name: &str) {
        if let Some(index) = self
            .zones
            .iter()
            .position(|zone| -> bool { zone.name == zone_name })
        {
            self.zones.remove(index);
        } else {
            println!(
                "Could not find Zone with id {} assigned to config {}",
                zone_name, self.name
            )
        }
    }
}

impl fmt::Display for MonitorConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: [{:#?}]", self.name, self.zones)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Zone {
    pub name: String,
    pub x: i16,
    pub y: i16,
    pub width: i16,
    pub height: i16,
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
        self.name != ""
            && x >= self.x
            && x <= self.x + self.width
            && y >= self.y
            && y <= self.y + self.height
    }

    pub fn get_sqr_dist_to(&self, x: i16, y: i16) -> u32 {
        let cp = self.get_center_point();
        (((x - cp.0) as i32).pow(2) + ((y - cp.1) as i32).pow(2))
            .try_into()
            .expect("Failed to calculate square distance")
    }

    pub fn get_area(&self) -> u32 {
        self.width as u32 * self.height as u32
    }
}

pub fn init_cfg_file<C: Connection>(conn: &C, root: u32) {
    let mut monitors = util::get_monitors(conn, root).unwrap();
    let mut monitor_configs = vec![];

    for monitor in &mut monitors {
        let zones = vec![];
        let monitor_config = MonitorConfig {
            name: monitor.name.clone(),
            zones,
        };
        monitor_configs.push(monitor_config.clone());
        monitor.config = Some(monitor_config);
    }

    let config = Config {
        monitors,
        monitor_configs,
        alpha: 0.5,
        line_thickness: 3,
    };

    let path = get_config_path();
    let dir = path.parent().unwrap_or(Path::new("/"));
    fs::create_dir_all(&dir).expect(&format!("Failed to create dir at {:#?}", dir));
    let mut cfg_file = File::create(&path).expect(&format!(
        "Failed to create fresh config file at {:#?}.",
        path
    ));
    let data = serde_json::to_vec(&config).unwrap();
    _ = cfg_file
        .write(&data)
        .expect(&format!("Failed to write config file at {:#?}", path));
}

pub fn load_cfg_file() -> Config {
    let path = get_config_path();
    let data = read(&path).expect(&format!("Failed to read config file at {:#?}", path));
    serde_json::from_slice(&data).unwrap()
}

pub fn save_cfg_file(config: &Config) {
    let path = get_config_path();
    let data = serde_json::to_vec(config).unwrap();
    let mut file =
        File::create(&path).expect(&format!("Failed to open config file at {:#?}", path));
    _ = file
        .write(&data)
        .expect(&format!("Failed to write config file at {:#?}", path));
}
