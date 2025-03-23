use x11rb::{
    COPY_DEPTH_FROM_PARENT,
    connection::Connection,
    errors::ReplyOrIdError,
    protocol::{
        Event,
        shape::{self, ConnectionExt as _},
        xproto::{PixmapWrapper, *},
    },
    wrapper::ConnectionExt as _,
};

use crate::{config::Zone, util::Monitor};

pub struct AtomContainer {
    pub wm_protocols: u32,
    pub wm_delete_window: u32,
    pub net_wm_state: u32,
    pub net_wm_state_above: u32,
    pub motif_wm_hints: u32,
    pub wm_window_opacity: u32,
    pub no_decorations_hint: [u32; 5],
}

impl AtomContainer {
    pub const NO_DECORATIONS_HINT: [u32; 5] = [2, 0, 0, 0, 0];

    pub fn new<'a, C: Connection + 'a>(conn: &'a C) -> Result<Self, ReplyOrIdError> {
        let wm_protocols = conn.intern_atom(false, b"WM_PROTOCOLS")?.reply()?.atom;
        let wm_delete_window = conn.intern_atom(false, b"WM_DELETE_WINDOW")?.reply()?.atom;
        let net_wm_state = conn.intern_atom(false, b"_NET_WM_STATE")?.reply()?.atom;
        let net_wm_state_above = conn
            .intern_atom(false, b"_NET_WM_STATE_ABOVE")?
            .reply()?
            .atom;
        let motif_wm_hints = conn.intern_atom(false, b"_MOTIF_WM_HINTS")?.reply()?.atom;
        let wm_window_opacity = conn
            .intern_atom(false, b"_NET_WM_WINDOW_OPACITY")?
            .reply()?
            .atom;
        Ok(Self {
            wm_protocols,
            wm_delete_window,
            net_wm_state,
            net_wm_state_above,
            motif_wm_hints,
            wm_window_opacity,
            no_decorations_hint: Self::NO_DECORATIONS_HINT,
        })
    }
}

pub struct Colors<'a, C: Connection + 'a> {
    white: GcontextWrapper<&'a C>,
    black: GcontextWrapper<&'a C>,
}

impl<'a, C: Connection + 'a> Colors<'a, C> {
    pub fn new(conn: &'a C, win_id: Window, screen: &Screen) -> Result<Self, ReplyOrIdError> {
        let white_gcw = GcontextWrapper::create_gc(
            conn,
            win_id,
            &CreateGCAux::new()
                .graphics_exposures(0)
                .foreground(screen.white_pixel),
        )?;
        let black_gcw = GcontextWrapper::create_gc(
            conn,
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

pub struct OverlayWindow<'a, 'b, 'c, C: Connection + 'a> {
    conn: &'a C,
    screen: &'b Screen,
    monitor: &'b Monitor,
    zones: Vec<Zone>,
    pixmap: Option<PixmapWrapper<&'a C>>,
    alpha: f32,
    win_id: Window,
    atoms: &'b AtomContainer,
    colors: Option<Colors<'a, C>>,
    active_zone: Option<&'c Zone>,
}

impl<'a, 'b, 'c, C: Connection + 'a> OverlayWindow<'a, 'b, 'c, C> {
    pub fn new(
        conn: &'a C,
        screen: &'b Screen,
        monitor: &'b Monitor,
        zones: Vec<Zone>,
        atoms: &'b AtomContainer,
        alpha: f32,
    ) -> Result<Self, ReplyOrIdError> {
        assert!(
            conn.extension_information(shape::X11_EXTENSION_NAME)
                .unwrap()
                .is_some(),
            "Shape extension is required."
        );

        let win_id = conn.generate_id()?;

        let alpha = alpha.clamp(0.0, 1.0);

        Ok(OverlayWindow {
            conn,
            screen,
            monitor,
            zones,
            pixmap: None,
            alpha,
            win_id,
            atoms,
            colors: None,
            active_zone: None,
        })
    }

    pub fn setup_window(&mut self) -> Result<&Self, ReplyOrIdError> {
        let win_aux = CreateWindowAux::new()
            .event_mask(EventMask::EXPOSURE | EventMask::STRUCTURE_NOTIFY)
            .background_pixel(self.screen.white_pixel);
        let opacity: u32 = (self.alpha * u32::MAX as f32) as u32;
        let wm_normal_hints = [
            15,                         // Flags: PMinSize | PMaxSize
            self.monitor.width as u32,  // min width
            self.monitor.height as u32, // min height
            self.monitor.width as u32,  // max width
            self.monitor.height as u32, // max height
            0,
            0,
            0,
            0,
            0,
        ];

        self.conn.create_window(
            COPY_DEPTH_FROM_PARENT,
            self.win_id,
            self.screen.root,
            self.monitor.x,
            self.monitor.y,
            self.monitor.width,
            self.monitor.height,
            0,
            WindowClass::INPUT_OUTPUT,
            0,
            &win_aux,
        )?;

        self.conn.change_property32(
            PropMode::REPLACE,
            self.win_id,
            self.atoms.wm_protocols,
            AtomEnum::ATOM,
            &[self.atoms.wm_delete_window],
        )?;

        self.conn.change_property32(
            PropMode::REPLACE,
            self.win_id,
            self.atoms.motif_wm_hints,
            AtomEnum::ATOM,
            &self.atoms.no_decorations_hint,
        )?;

        self.conn.change_property32(
            PropMode::REPLACE,
            self.win_id,
            self.atoms.wm_window_opacity,
            AtomEnum::CARDINAL,
            &[opacity],
        )?;

        self.conn.change_property32(
            PropMode::REPLACE,
            self.win_id,
            AtomEnum::WM_NORMAL_HINTS,
            AtomEnum::CARDINAL,
            &wm_normal_hints,
        )?;

        self.pixmap = Some(PixmapWrapper::create_pixmap(
            self.conn,
            self.screen.root_depth,
            self.win_id,
            self.monitor.width,
            self.monitor.height,
        )?);

        self.colors = Some(Colors::new(self.conn, self.win_id, self.screen)?);

        self.conn.flush()?;

        Ok(self)
    }

    pub fn update(&self) -> Result<bool, ReplyOrIdError> {
        self.conn.flush()?;

        let pixmap = self.pixmap.as_ref().expect("Pixmap not setup");
        let colors = self.colors.as_ref().expect("Colors not setup");

        let mut need_redraw = false;
        let mut shutdown = false;

        let event = self.conn.wait_for_event()?;
        let mut event_op = Some(event);
        while let Some(event) = event_op {
            match event {
                Event::Expose(e) => {
                    if e.count == 0 {
                        need_redraw = true;
                    }
                }
                Event::ConfigureNotify(_) => (),
                Event::MotionNotify(_) => (),
                Event::MapNotify(_) => {
                    self.set_always_on_top()?;
                    self.shape_window()?;
                    need_redraw = true;
                }
                Event::ClientMessage(e) => {
                    let data = e.data.as_data32();
                    if e.format == 32
                        && e.window == self.win_id
                        && data[0] == self.atoms.wm_delete_window
                    {
                        println!("Shutting down");
                        shutdown = true;
                    }
                }
                Event::Error(e) => {
                    println!("Got error {:?}", e);
                }
                e => {
                    println!("Got unhandled event {:?}", e);
                }
            }

            event_op = self.conn.poll_for_event()?;
        }
        if need_redraw {
            self.draw_zones()?;
            self.conn.copy_area(
                pixmap.pixmap(),
                self.win_id,
                colors.white.gcontext(),
                self.monitor.x,
                self.monitor.y,
                self.monitor.x,
                self.monitor.x,
                self.monitor.width,
                self.monitor.height,
            )?;
            self.conn.flush()?;
        }
        Ok(shutdown)
    }

    pub fn show(&self) -> Result<(), ReplyOrIdError> {
        self.conn.map_window(self.win_id)?;
        self.conn.flush()?;
        Ok(())
    }

    pub fn hide(&self) -> Result<(), ReplyOrIdError> {
        self.conn.unmap_window(self.win_id)?;
        self.conn.flush()?;
        Ok(())
    }

    pub fn find_active_zone(&'c mut self, x: i16, y: i16) {
        for zone in &self.zones {
            if x >= zone.x && x <= zone.x + zone.width && y >= zone.y && y <= zone.y + zone.height {
                self.active_zone = Some(zone);
                return;
            }
        }
        self.active_zone = None;
    }

    fn draw_zones(&self) -> Result<(), ReplyOrIdError> {
        let thickness = 5_u16;

        let colors = self.colors.as_ref().expect("Colors not setup");

        for zone in &self.zones {
            let top = Rectangle {
                x: zone.x,
                y: zone.y,
                width: zone.width as u16,
                height: thickness,
            };

            let left = Rectangle {
                x: zone.x,
                y: zone.y,
                width: thickness,
                height: zone.height as u16,
            };

            let right = Rectangle {
                x: zone.x + zone.width - thickness as i16,
                y: zone.y,
                width: thickness,
                height: zone.height as u16,
            };

            let bottom = Rectangle {
                x: zone.x,
                y: zone.y + zone.height - thickness as i16,
                width: zone.width as u16,
                height: thickness,
            };

            let bg = Rectangle {
                x: zone.x,
                y: zone.y,
                width: zone.width as u16,
                height: zone.height as u16,
            };

            self.conn
                .poly_fill_rectangle(self.win_id, colors.black.gcontext(), &[bg])?;
            self.conn.poly_fill_rectangle(
                self.win_id,
                colors.white.gcontext(),
                &[top, left, right, bottom],
            )?;
        }

        self.draw_active_zone()?;

        Ok(())
    }

    fn draw_active_zone(&self) -> Result<(), ReplyOrIdError> {
        let colors = self.colors.as_ref().expect("Colors not setup");

        if let Some(zone) = self.active_zone {
            let rect = Rectangle {
                x: zone.x,
                y: zone.y,
                width: zone.width as u16,
                height: zone.height as u16,
            };

            self.conn
                .poly_fill_rectangle(self.win_id, colors.white.gcontext(), &[rect])?;
        }
        Ok(())
    }

    fn shape_window(&self) -> Result<(), ReplyOrIdError> {
        let pixmap = PixmapWrapper::create_pixmap(
            self.conn,
            1,
            self.win_id,
            self.monitor.width,
            self.monitor.height,
        )?;

        // Make transparent
        let gc = GcontextWrapper::create_gc(
            self.conn,
            pixmap.pixmap(),
            &CreateGCAux::new().graphics_exposures(0).foreground(0),
        )?;

        let rect = Rectangle {
            x: self.monitor.x,
            y: self.monitor.y,
            width: self.monitor.width as u16,
            height: self.monitor.height as u16,
        };
        self.conn
            .poly_fill_rectangle(pixmap.pixmap(), gc.gcontext(), &[rect])?;

        let values = ChangeGCAux::new().foreground(1);
        self.conn.change_gc(gc.gcontext(), &values)?;
        self.draw_zones()?;

        self.conn.shape_mask(
            shape::SO::SET,
            shape::SK::BOUNDING,
            self.win_id,
            0,
            0,
            &pixmap,
        )?;

        Ok(())
    }

    fn set_always_on_top(&self) -> Result<(), ReplyOrIdError> {
        let data = ClientMessageData::from([1, self.atoms.net_wm_state_above, 0, 0, 0]); // 1 = _NET_WM_STATE_ADD

        let msg = ClientMessageEvent {
            response_type: CLIENT_MESSAGE_EVENT,
            format: 32,
            window: self.win_id,
            type_: self.atoms.net_wm_state,
            data,
            sequence: 0,
        };

        self.conn.send_event(
            false,
            self.screen.root,
            EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
            msg,
        )?;

        self.conn.flush()?;
        Ok(())
    }
}

pub struct Overlay<'a, 'b, 'c, C: Connection + 'a> {
    conn: &'a C,
    monitors: Vec<Monitor>,
    zones: Vec<Zone>,
    colors: Colors<'a, C>,
    windows: Vec<OverlayWindow<'a, 'b, 'c, C>>,
}

impl<'a, C: Connection + 'a> Overlay<'a, '_, '_, C> {
    pub fn new() {}

    pub fn tick() {}

    pub fn show() {}

    pub fn hide() {}

    fn find_active_monitor() {}
}
