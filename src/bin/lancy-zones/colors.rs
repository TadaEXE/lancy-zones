use std::rc::Rc;

use x11rb::{
    connection::Connection, errors::ReplyOrIdError, protocol::xproto::*,
};

pub struct Colors<C: Connection> {
    pub white: GcontextWrapper<Rc<C>>,
    pub black: GcontextWrapper<Rc<C>>,
}

impl<C: Connection> Colors<C> {
    pub fn new(conn: Rc<C>, win_id: Window, screen: &Screen) -> Result<Self, ReplyOrIdError> {
        let white_gcw = GcontextWrapper::create_gc(
            conn.clone(),
            win_id,
            &CreateGCAux::new()
                .graphics_exposures(0)
                .foreground(screen.white_pixel),
        )?;
        let black_gcw = GcontextWrapper::create_gc(
            conn.clone(),
            win_id,
            &CreateGCAux::new()
                .graphics_exposures(0)
                .foreground(screen.black_pixel),
        )?;

        Ok(Colors {
            white: white_gcw,
            black: black_gcw,
        })
    }
}
