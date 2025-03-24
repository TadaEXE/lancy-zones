use std::rc::Rc;

use config::{Config, MonitorConfig, WmCounterPadding};
use x11rb::connection::Connection;

mod config;
mod overlay;
mod util;

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

    let zones = vec![zone1, zone2];
    let monitors = util::get_monitors(&conn, screen.root).unwrap();

    let cp = WmCounterPadding {
        x: -24,
        y: -23,
        w: 53,
        h: 52,
    };

    let mc = MonitorConfig {
        name: "test".to_string(),
        zones,
        monitor: monitors[2].clone(),
        counter_padding: cp,
    };

    let config = Config {
        monitor_configs: vec![mc],
    };

    let atoms = Rc::new(AtomContainer::new(&conn).unwrap());
    let screen = Rc::new(screen);
    let mut overlay = Overlay::new(conn, &config, screen, atoms);
    overlay.listen().unwrap();
    // let mut overlay = OverlayWindow::new(&conn, &screen, &monitors[2], zones, &atoms, 0.5, 2)
    //     .unwrap()
    //     .setup_window()
    //     .unwrap();
    //
    // overlay.show().unwrap();
    // loop {
    //     overlay.update().unwrap();
    // }

    // let mut o = overlayold::Overlay::new(&conn, screen_num, zones, &monitors[0]).unwrap();
    // o.run_until(|| true).unwrap();

    // system_listener::listen().unwrap();
    // let monitors = util::get_monitors(&conn, *root_window).unwrap();
    // dbg!(monitors);
}
