use std::{fs, path::Path, rc::Rc};

use clap::error;
use x11rb::{connection::Connection, protocol::xproto::Screen, rust_connection::RustConnection};

use lancy_zones::{
    config::{self, load_cfg_file},
    util,
};

fn make_conn() -> (Rc<RustConnection>, Rc<Screen>) {
    let (conn, screen_num) = x11rb::connect(None).unwrap();
    let screen = &conn.setup().roots[screen_num];
    let screen = Rc::new(screen.to_owned());
    let conn = Rc::new(conn);
    (conn, screen)
}

fn get_monitor_by_name(monitor_name: &str) -> config::Monitor {
    let (conn, screen) = make_conn();
    let monitors = util::get_monitors(&conn, screen.root).expect("Could not fetch monitors");
    monitors
        .iter()
        .find(|monitor| -> bool { monitor.name == monitor_name })
        .cloned()
        .expect(&std::format!(
            "Could not find monitor with name {}",
            monitor_name
        ))
}

fn get_monitor_of_assigned_config<'a>(
    config: &'a mut config::Config,
    config_name: &str,
) -> Option<&'a mut config::Monitor> {
    config.monitors.iter_mut().find(|monitor| -> bool {
        if let Some(cfg) = &monitor.config {
            cfg.name == config_name
        } else {
            false
        }
    })
}

pub fn list_cmd() {
    println!("{}", config::load_cfg_file());
}

pub fn reinit_cmd() {
    let path = config::get_config_path();
    if path.exists() {
        fs::remove_file(&path).expect(&std::format!(
            "Could not delete exisiting config file at {}",
            path.to_str().unwrap()
        ));
    }
    let (conn, screen) = make_conn();
    config::init_cfg_file(&conn, screen.root);
}

pub fn create_config_cmd(config_name: &str) {
    let mut config = config::load_cfg_file();
    match config.get_monitor_config(config_name) {
        Some(_) => panic!("{} already exists", config_name),
        _ => {
            let new_mc = config::MonitorConfig {
                name: config_name.to_string(),
                zones: vec![],
            };

            config.monitor_configs.push(new_mc);
            config::save_cfg_file(&config);
        }
    }
}

pub fn remove_config_cmd(config_name: &str) {
    let mut config = config::load_cfg_file();
    if let Some(index) = config
        .monitor_configs
        .iter()
        .position(|mc| -> bool { mc.name == config_name })
    {
        if let Some(monitor) = get_monitor_of_assigned_config(&mut config, config_name) {
            monitor.config = None;
        }
        config.monitor_configs.remove(index);
    } else {
        panic!("{} does not exist", config_name);
    }
    config::save_cfg_file(&config);
}

pub fn add_zone_cmd(config_name: &str, zone_name: &str, x: i16, y: i16, width: i16, height: i16) {
    let mut config = config::load_cfg_file();
    let new_zone = config::Zone {
        name: zone_name.to_string(),
        x,
        y,
        width,
        height,
    };

    if let Some(monitor_config) = config.get_monitor_config_mut(config_name) {
        monitor_config.add_zone(new_zone);
    } else {
        panic!("{} does not exist", config_name);
    }

    let monitor_config = config.get_monitor_config(config_name).unwrap().clone();
    if let Some(monitor) = get_monitor_of_assigned_config(&mut config, config_name) {
        monitor.config = Some(monitor_config);
    }
    config::save_cfg_file(&config);
}

pub fn remove_zone_cmd(config_name: &str, zone_name: &str) {
    let mut config = config::load_cfg_file();
}

pub fn assign_cmd(monitor_name: &str, config_name: &str) {
    let mut config = config::load_cfg_file();
    let monitor_config = config
        .get_monitor_config(config_name)
        .expect(&std::format!("{} does not exist", config_name))
        .clone();
    match config
        .monitors
        .iter_mut()
        .find(|monitor| -> bool { monitor.name == monitor_name })
    {
        Some(monitor) => monitor.config = Some(monitor_config),
        _ => panic!("{} does not exist", monitor_name),
    }
    config::save_cfg_file(&config);
}

pub fn unassing_cmd(monitor_name: &str) {
    let mut config = config::load_cfg_file();
    match config
        .monitors
        .iter_mut()
        .find(|monitor| -> bool { monitor.name == monitor_name })
    {
        Some(monitor) => monitor.config = None,
        _ => panic!("{} does not exist", monitor_name),
    }
    config::save_cfg_file(&config);
}
