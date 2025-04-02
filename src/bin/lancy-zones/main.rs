mod overlay;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use config::{Config, MonitorConfig};
use lancy_zones::config::{init_cfg_file, load_cfg_file};
use x11rb::connection::Connection;

use lancy_zones::{config, util};

use crate::config::Zone;
use crate::overlay::{AtomContainer, Overlay};

fn main() {
    let zone1 = Zone {
        id: 0,
        x: 0,
        y: 0,
        width: 800,
        height: 1080,
    };
    let zone2 = Zone {
        id: 1,
        x: 800,
        y: 0,
        width: 900,
        height: 1080,
    };

    let (conn, screen_num) = x11rb::connect(None).unwrap();
    let conn = Rc::new(conn);
    let screen = conn.setup().roots[screen_num].clone();

    let path = Path::new("~/.config/lancy-zones/config.json");
    if !path.exists() {
        init_cfg_file(&path, &conn, screen.root);
    }
    let config = load_cfg_file(&path);

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
    let mut overlay = Overlay::new(conn, &config, screen, atoms);
    overlay.listen().unwrap();
}
