use std::error::Error;
use std::str;

use serde::Deserialize;
use serde::Serialize;
use x11rb::connection::Connection;
use x11rb::errors::ReplyOrIdError;
use x11rb::protocol::randr;
use x11rb::protocol::xproto::*;

use crate::config::Zone;

pub fn scan_windows<C: Connection>(
    con: &C,
    screen: &Screen,
    condition: fn(MapState) -> bool,
) -> Result<Vec<u32>, ReplyOrIdError> {
    let tree_reply = con.query_tree(screen.root)?.reply()?;

    let mut cookies = Vec::with_capacity(tree_reply.children.len());
    for win in tree_reply.children {
        let attr = con.get_window_attributes(win)?;
        let geom = con.get_geometry(win)?;
        cookies.push((win, attr, geom));
    }

    let mut all_windows: Vec<u32> = Vec::with_capacity(cookies.len());

    for (win, attr, geom) in cookies {
        if let (Ok(attr), Ok(_geom)) = (attr.reply(), geom.reply()) {
            if !attr.override_redirect && condition(attr.map_state) {
                let win_name = con
                    .get_property(false, win, AtomEnum::WM_NAME, AtomEnum::STRING, 0, u32::MAX)?
                    .reply()?
                    .value;
                all_windows.push(win);
                println!("Found window: {}", str::from_utf8(&win_name).unwrap());
            }
        }
    }

    Ok(all_windows)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Monitor {
    pub name: String,
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

pub fn get_monitors<C: Connection>(
    conn: &C,
    root_window: Window,
) -> Result<Vec<Monitor>, ReplyOrIdError> {
    let mut monitors = Vec::new();
    let screen_resources = randr::get_screen_resources(conn, root_window)?.reply()?;
    for s in screen_resources.outputs {
        // can't combine these if statements because it's unstable
        if let Ok(output_info) =
            randr::get_output_info(&conn, s, screen_resources.config_timestamp)?.reply()
        {
            if output_info.connection == randr::Connection::CONNECTED {
                match randr::get_crtc_info(
                    &conn,
                    output_info.crtc,
                    screen_resources.config_timestamp,
                )?
                .reply()
                {
                    Ok(crtc_info) => {
                        monitors.push(Monitor {
                            name: String::from_utf8(output_info.name).unwrap(),
                            x: crtc_info.x.try_into().unwrap(),
                            y: crtc_info.y.try_into().unwrap(),
                            width: crtc_info.width,
                            height: crtc_info.height,
                        });
                    }
                    Err(e) => {
                        dbg!(e);
                    }
                }
            }
        }
    }

    Ok(monitors)
}
