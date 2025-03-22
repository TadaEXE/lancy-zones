mod config;
mod overlay;
mod util;
mod system_listener;

// use crate::config::Zone;
// use crate::overlay::Overlay;

fn main() {
    // let zone1 = Zone {
    //     x: 0,
    //     y: 0,
    //     width: 2520,
    //     height: 1440,
    // };
    // let zone2 = Zone {
    //     x: 2520,
    //     y: 0,
    //     width: 2520,
    //     height: 1440,
    // };
    //
    // let zones = vec![zone1, zone2];
    // let (conn, screen_num) = x11rb::connect(None).unwrap();
    // let mut overlay = Overlay::new(&conn, screen_num, zones).unwrap();
    // overlay.run_until(|| true).unwrap();
    //
    system_listener::listen().unwrap();
}
