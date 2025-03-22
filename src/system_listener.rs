use std::error::Error;
use x11rb::connection::Connection;
use x11rb::protocol::{Event, xproto::*};

fn ctrl_pressed<C: Connection>(conn: &C, root_window: Window) -> bool {
    if let Ok(reply) = conn.query_pointer(root_window).unwrap().reply() {
        reply.mask & KeyButMask::CONTROL != KeyButMask::from(0_u16)
    } else {
        false
    }
}

pub fn listen() -> Result<(), Box<dyn Error>> {
    let (conn, screen_num) = x11rb::connect(None)?;
    let screen = &conn.setup().roots[screen_num];
    let root_window = screen.root;

    conn.change_window_attributes(
        root_window,
        &ChangeWindowAttributesAux::new()
            .event_mask(EventMask::SUBSTRUCTURE_NOTIFY | EventMask::STRUCTURE_NOTIFY),
    )?;
    conn.flush()?;

    loop {
        let event = conn.wait_for_event()?;
        match event {
            Event::ConfigureNotify(configure_event) => {
                let ctrl = ctrl_pressed(&conn, root_window);
                println!(
                    "Window {} moved to: ({}, {}) ctrl_pressed: {}",
                    configure_event.window, configure_event.x, configure_event.y, ctrl
                );
            }
            _ => {}
        }
    }
}
