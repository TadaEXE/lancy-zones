mod atoms;
mod colors;
mod overlay;
use std::path::Path;
use std::rc::Rc;

use lancy_zones::config::{init_cfg_file, load_cfg_file};
use x11rb::connection::Connection;

use lancy_zones::{config, util};

use crate::atoms::AtomContainer;
use crate::overlay::Overlay;

fn main() {
    let (conn, screen_num) = x11rb::connect(None).unwrap();
    let conn = Rc::new(conn);
    let screen = conn.setup().roots[screen_num].clone();

    let path = Path::new("~/.config/lancy-zones/config.json");
    if !path.exists() {
        init_cfg_file(&path, &conn, screen.root);
    }
    let config = Rc::new(load_cfg_file(&path));

    // let zones = vec![zone1, zone2];
    // let monitors = util::get_monitors(&conn, screen.root).unwrap();
    // let mc = MonitorConfig {
    //     name: "test".to_string(),
    //     zones,
    //     monitor: monitors[2].clone(),
    // };
    //
    // let config = Config {
    //     monitor_configs: vec![mc],
    // };

    let atoms = Rc::new(AtomContainer::new(&conn).unwrap());
    let screen = Rc::new(screen);
    let mut overlay = Overlay::new(conn, screen.clone(), atoms, config.clone())
        .init()
        .unwrap();
    _ = overlay.listen();
    // println!("{}x{}", screen.width_in_pixels, screen.height_in_pixels);
    // for m in &config.monitor_configs {
    //     println!("{}", m.monitor);
    // }
}
