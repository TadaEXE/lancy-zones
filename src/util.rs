use std::str;

use x11rb::connection::Connection;
use x11rb::errors::ReplyOrIdError;
use x11rb::protocol::xproto::*;

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


