use std::{rc::Rc, thread::sleep, time::Duration};

use x11rb::{
    COPY_DEPTH_FROM_PARENT,
    connection::Connection,
    errors::ReplyOrIdError,
    properties::{WmSizeHints, WmSizeHintsSpecification},
    protocol::{
        Event,
        shape::{self, ConnectionExt as _},
        xinput::{ConnectionExt as _, Device, XIEventMask},
        xproto::{PixmapWrapper, *},
    },
    reexports::x11rb_protocol::protocol::xinput,
    wrapper::ConnectionExt as _,
};

use crate::{
    config::{Config, Zone},
    util::Monitor,
};

pub struct AtomContainer {
    pub wm_protocols: u32,
    pub wm_delete_window: u32,
    pub net_wm_state: u32,
    pub net_wm_state_above: u32,
    pub motif_wm_hints: u32,
    pub wm_window_opacity: u32,
    pub wm_type: u32,
    pub wm_type_notification: u32,
    pub net_extents: u32,
    pub gtk_extents: u32,
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
        let wm_type = conn.intern_atom(false, b"_NET_WM_TYPE")?.reply()?.atom;
        let wm_type_notification = conn
            .intern_atom(false, b"_NET_WM_WINDOW_TYPE_NOTIFICATION")?
            .reply()?
            .atom;
        let net_extents = conn
            .intern_atom(false, b"_NET_FRAME_EXTENTS")?
            .reply()?
            .atom;
        let gtk_extents = conn
            .intern_atom(false, b"_GTK_FRAME_EXTENTS")?
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
            wm_type,
            wm_type_notification,
            net_extents,
            gtk_extents,
        })
    }
}

pub struct Colors<C: Connection> {
    white: GcontextWrapper<Rc<C>>,
    black: GcontextWrapper<Rc<C>>,
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

struct OverlayWindow<'a, C: Connection> {
    conn: Rc<C>,
    screen: Rc<Screen>,
    monitor: &'a Monitor,
    zones: &'a [Zone],
    pixmap: Option<PixmapWrapper<Rc<C>>>,
    alpha: f32,
    win_id: Window,
    atoms: Rc<AtomContainer>,
    colors: Option<Colors<C>>,
    active_zone: Option<usize>,
    line_thickness: u16,
}

impl<'a, C: Connection> OverlayWindow<'a, C> {
    pub fn new(
        conn: Rc<C>,
        screen: Rc<Screen>,
        monitor: &'a Monitor,
        zones: &'a [Zone],
        atoms: Rc<AtomContainer>,
        alpha: f32,
        line_thickness: u16,
    ) -> Result<Self, ReplyOrIdError> {
        assert!(
            conn.extension_information(shape::X11_EXTENSION_NAME)
                .unwrap()
                .is_some(),
            "Shape extension is required."
        );
        assert!(
            conn.extension_information(xinput::X11_EXTENSION_NAME)
                .unwrap()
                .is_some(),
            "XInput extension is required."
        );

        for zone in zones {
            assert!(
                zone.x < monitor.width.try_into().unwrap()
                    && zone.y < monitor.height.try_into().unwrap(),
                "Zone {} starts out of bounds at (x: {}, y: {}) for monitor {} {}x{}",
                zone.id,
                zone.x,
                zone.y,
                monitor.name,
                monitor.width,
                monitor.height
            );
            assert!(
                zone.x + zone.width <= monitor.width.try_into().unwrap()
                    && zone.y + zone.height <= monitor.height.try_into().unwrap(),
                "Zone {} ends out of bounds at (x: {}, y: {}) for monitor {} {}x{}",
                zone.id,
                zone.x + zone.width,
                zone.y + zone.height,
                monitor.name,
                monitor.width,
                monitor.height
            );
        }

        let win_id = conn.generate_id()?;

        let alpha = alpha.clamp(0.0, 1.0);

        dbg!(monitor);

        Ok(OverlayWindow {
            conn,
            screen,
            monitor,
            zones: &zones,
            pixmap: None,
            alpha,
            win_id,
            atoms,
            colors: None,
            active_zone: None,
            line_thickness,
        })
    }

    pub fn setup_window(mut self) -> Result<Self, ReplyOrIdError> {
        let win_aux = CreateWindowAux::new()
            .event_mask(EventMask::EXPOSURE | EventMask::STRUCTURE_NOTIFY)
            .background_pixel(self.screen.white_pixel)
            .override_redirect(1);
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

        let title = "lancy-zones";
        self.conn.change_property8(
            PropMode::REPLACE,
            self.win_id,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            title.as_bytes(),
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

        self.conn.change_property32(
            PropMode::REPLACE,
            self.win_id,
            self.atoms.wm_type,
            AtomEnum::ATOM,
            &[self.atoms.wm_type_notification],
        )?;

        self.conn.change_property32(
            PropMode::APPEND,
            self.win_id,
            self.atoms.net_wm_state,
            AtomEnum::ATOM,
            &[self.atoms.net_wm_state_above],
        )?;

        self.pixmap = Some(PixmapWrapper::create_pixmap(
            self.conn.clone(),
            self.screen.root_depth,
            self.win_id,
            self.monitor.width,
            self.monitor.height,
        )?);

        self.conn.shape_mask(
            shape::SO::SET,
            shape::SK::INPUT,
            self.win_id,
            0,
            0,
            self.pixmap.as_ref().unwrap().pixmap(),
        )?;

        self.colors = Some(Colors::new(self.conn.clone(), self.win_id, &*self.screen)?);

        self.conn.flush()?;

        Ok(self)
    }

    pub fn snap_to_zone(&mut self, win: u32) -> Result<(), ReplyOrIdError> {
        if let Some(zone) = self.active_zone {
            let zone = &self.zones[zone];
            let conf = ConfigureWindowAux::new()
                .x(i32::from(zone.x + self.monitor.x))
                .y(i32::from(zone.y + self.monitor.y))
                .width(u32::try_from(zone.width).unwrap())
                .height(u32::try_from(zone.height).unwrap())
                .stack_mode(StackMode::ABOVE);

            self.disable_window_padding(win)?;

            self.conn.change_window_attributes(
                win,
                &ChangeWindowAttributesAux::new().win_gravity(Gravity::NORTH_WEST),
            )?;
            println!("Snapping to {:?} on monitor {:?}", zone, self.monitor);
            self.conn.configure_window(win, &conf)?;
            self.conn.flush()?;
            self.active_zone = None;
        }
        Ok(())
    }

    fn disable_window_padding(&self, win: u32) -> Result<(), ReplyOrIdError> {
        let no_extents = [0, 0, 0, 0];
        let _ = self.conn.change_property32(
            PropMode::REPLACE,
            win,
            self.atoms.net_extents,
            AtomEnum::CARDINAL,
            &no_extents,
        );
        let _ = self.conn.change_property32(
            PropMode::REPLACE,
            win,
            self.atoms.gtk_extents,
            AtomEnum::CARDINAL,
            &no_extents,
        );
        Ok(())
    }

    pub fn show(&self) -> Result<(), ReplyOrIdError> {
        self.conn.map_window(self.win_id)?;
        self.move_window_to_monitor()?;
        self.conn.flush()?;
        Ok(())
    }

    pub fn hide(&self) -> Result<(), ReplyOrIdError> {
        self.conn.unmap_window(self.win_id)?;
        self.conn.flush()?;
        Ok(())
    }

    pub fn find_active_zone(&mut self, x: i16, y: i16) {
        let (local_x, local_y) = self.monitor.to_local_space(x, y);
        if self.monitor.coords_inside(x, y) {
            self.active_zone = self.get_closest_center(local_x, local_y);
        } else {
            self.active_zone = None;
        }
        self.shape_window().unwrap();
        self.draw_zones(
            self.win_id,
            self.colors.as_ref().unwrap().white.gcontext(),
            self.colors.as_ref().unwrap().black.gcontext(),
        )
        .unwrap();
        // self.draw_snap_direction_indicator(
        //     local_x,
        //     local_y,
        //     self.colors.as_ref().unwrap().black.gcontext(),
        // )
        // .unwrap();
    }

    fn draw_snap_direction_indicator(
        &self,
        x: i16,
        y: i16,
        gc: Gcontext,
    ) -> Result<(), ReplyOrIdError> {
        if let Some(active_zone) = self.active_zone {
            let active_zone = &self.zones[active_zone];
            let p_cursor = Point { x, y };
            let (cx, cy) = active_zone.get_center_point();
            let p_center = Point { x: cx, y: cy };
            self.conn
                .poly_line(CoordMode::ORIGIN, self.win_id, gc, &[p_center, p_cursor])?;
        }
        Ok(())
    }

    fn get_closest_center(&self, x: i16, y: i16) -> Option<usize> {
        let mut len = f64::MAX;
        let mut i: usize = 0;
        let mut res = None;
        for zone in self.zones {
            let (cx, cy) = zone.get_center_point();
            let cur_len = f64::sqrt(
                f64::try_from(((x - cx) as i32).pow(2) + ((y - cy) as i32).pow(2)).unwrap(),
            );
            if cur_len < len {
                len = cur_len;
                res = Some(i);
            }
            i += 1;
        }

        res
    }

    fn move_window_to_monitor(&self) -> Result<(), ReplyOrIdError> {
        let config_aux = ConfigureWindowAux::new()
            .x(self.monitor.x as i32)
            .y(self.monitor.y as i32);
        self.conn.configure_window(self.win_id, &config_aux)?;
        Ok(())
    }

    fn draw_zones(&self, win_id: Window, c1: Gcontext, c2: Gcontext) -> Result<(), ReplyOrIdError> {
        for zone in self.zones {
            let top = Rectangle {
                x: zone.x,
                y: zone.y,
                width: zone.width as u16,
                height: self.line_thickness,
            };

            let left = Rectangle {
                x: zone.x,
                y: zone.y,
                width: self.line_thickness,
                height: zone.height as u16,
            };

            let right = Rectangle {
                x: zone.x + zone.width - self.line_thickness as i16,
                y: zone.y,
                width: self.line_thickness,
                height: zone.height as u16,
            };

            let bottom = Rectangle {
                x: zone.x,
                y: zone.y + zone.height - self.line_thickness as i16,
                width: zone.width as u16,
                height: self.line_thickness,
            };

            let bg = Rectangle {
                x: zone.x,
                y: zone.y,
                width: zone.width as u16,
                height: zone.height as u16,
            };

            self.conn.poly_fill_rectangle(win_id, c2, &[bg])?;
            self.conn
                .poly_fill_rectangle(win_id, c1, &[top, left, right, bottom])?;
        }

        self.draw_active_zone()?;

        Ok(())
    }

    fn draw_active_zone(&self) -> Result<(), ReplyOrIdError> {
        let colors = self.colors.as_ref().expect("Colors not setup");

        if let Some(zone) = self.active_zone {
            let zone = &self.zones[zone];
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
            self.conn.clone(),
            1,
            self.win_id,
            self.monitor.width,
            self.monitor.height,
        )?;
        // Make transparent
        let gc = GcontextWrapper::create_gc(
            self.conn.clone(),
            pixmap.pixmap(),
            &CreateGCAux::new().graphics_exposures(0).foreground(0),
        )?;
        let rect = Rectangle {
            x: 0,
            y: 0,
            width: self.monitor.width as u16,
            height: self.monitor.height as u16,
        };

        self.conn
            .poly_fill_rectangle(pixmap.pixmap(), gc.gcontext(), &[rect])?;

        let values = ChangeGCAux::new().foreground(1);

        self.conn.change_gc(gc.gcontext(), &values)?;
        self.draw_zones(pixmap.pixmap(), gc.gcontext(), gc.gcontext())?;

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
}

pub struct Overlay<'a, C: Connection> {
    conn: Rc<C>,
    windows: Vec<OverlayWindow<'a, C>>,
    screen: Rc<Screen>,
}

impl<'a, C: Connection> Overlay<'a, C> {
    pub fn new(
        conn: Rc<C>,
        config: &'a Config,
        screen: Rc<Screen>,
        atom_container: Rc<AtomContainer>,
    ) -> Self {
        let mut windows = Vec::new();

        for mc in &config.monitor_configs {
            let window = OverlayWindow::new(
                conn.clone(),
                screen.clone(),
                &mc.monitor,
                &mc.zones,
                atom_container.clone(),
                0.5,
                2,
            )
            .unwrap()
            .setup_window()
            .unwrap();
            windows.push(window);
        }
        Overlay {
            conn,
            windows,
            screen,
        }
    }

    pub fn listen(&mut self) -> Result<(), ReplyOrIdError> {
        self.conn.change_window_attributes(
            self.screen.root,
            &ChangeWindowAttributesAux::new()
                .event_mask(EventMask::SUBSTRUCTURE_NOTIFY | EventMask::STRUCTURE_NOTIFY),
        )?;
        self.conn.xinput_xi_select_events(
            self.screen.root,
            &[xinput::EventMask {
                deviceid: Device::ALL.into(),
                mask: vec![XIEventMask::RAW_KEY_RELEASE | XIEventMask::RAW_BUTTON_RELEASE],
            }],
        )?;
        self.conn.flush()?;

        let mut is_showing = false;
        let mut win: Option<u32> = None;
        loop {
            let event = self.conn.wait_for_event()?;
            let ctrl = self
                .button_pressed(KeyButMask::CONTROL)
                .unwrap_or(false);
            match event {
                Event::ConfigureNotify(e) => {
                    if ctrl {
                        if !is_showing {
                            is_showing = true;
                            self.show();
                        }
                        win = Some(e.window);
                        self.find_active_zone(e.x, e.y);
                    }
                }
                Event::XinputRawKeyRelease(e) => {
                    if e.detail == 37 && is_showing {
                        is_showing = false;
                        self.hide();
                    }
                }
                Event::XinputRawButtonRelease(e) => {
                    if e.detail == 1 {
                        if is_showing {
                            if ctrl {
                                if let Some(active_win) = win {
                                    self.snap_to_zone(active_win);
                                    win = None;
                                } else {
                                    println!("Got no win outer");
                                }
                            }
                            is_showing = false;
                            self.hide();
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn snap_to_zone(&mut self, win: u32) {
        for ow in &mut self.windows {
            ow.snap_to_zone(win).unwrap();
        }
    }

    fn button_pressed(&self, but: KeyButMask) -> Result<bool, ReplyOrIdError> {
        if let Ok(reply) = self.conn.query_pointer(self.screen.root)?.reply() {
            Ok(reply.mask & but != KeyButMask::from(0_u16))
        } else {
            Ok(false)
        }
    }

    fn find_active_zone(&mut self, x: i16, y: i16) {
        for ref mut win in &mut self.windows {
            win.find_active_zone(x, y);
        }
    }

    fn show(&self) {
        for win in &self.windows {
            win.show().unwrap();
        }
        self.conn.flush().unwrap();
    }

    fn hide(&self) {
        for win in &self.windows {
            win.hide().unwrap();
        }
        self.conn.flush().unwrap();
    }
}
