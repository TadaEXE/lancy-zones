use std::{cmp::Ordering, rc::Rc};

use x11rb::{
    COPY_DEPTH_FROM_PARENT,
    connection::Connection,
    errors::ReplyOrIdError,
    protocol::{
        Event,
        shape::{self, ConnectionExt as _},
        xinput::{ConnectionExt as _, Device, XIEventMask},
        xproto::{PixmapWrapper, *},
    },
    reexports::x11rb_protocol::protocol::xinput,
    wrapper::ConnectionExt as _,
};

use crate::{atoms::AtomContainer, colors::Colors};

use lancy_zones::config::{Config, Zone};

pub struct Overlay<C: Connection> {
    conn: Rc<C>,
    screen: Rc<Screen>,
    zones: Vec<Zone>,
    atoms: Rc<AtomContainer>,
    colors: Option<Colors<C>>,
    config: Rc<Config>,
    win_id: Window,
    active_zone: Option<usize>,
    pixmap: Option<PixmapWrapper<Rc<C>>>,
}

impl<C: Connection> Overlay<C> {
    pub fn new(
        conn: Rc<C>,
        screen: Rc<Screen>,
        atoms: Rc<AtomContainer>,
        config: Rc<Config>,
    ) -> Self {
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

        let mut zones = Vec::new();
        for monitor in &config.monitors {
            // add background zone for correct rendering
            zones.push(Zone {
                name: "".to_string(),
                x: monitor.x,
                y: monitor.y,
                width: monitor.width as i16,
                height: monitor.height as i16,
            });
            if let Some(config) = &monitor.config {
                for zone in &config.zones {
                    let trans_zone = Zone {
                        name: zone.name.clone(),
                        x: zone.x + monitor.x,
                        y: zone.y + monitor.y,
                        width: zone.width,
                        height: zone.height,
                    };
                    zones.push(trans_zone);
                }
            }
        }

        // Sort by biggest area first (ording::less). This helps rendering of zones that cover
        // eachother, because the common case is a zone covered by farction of itself
        zones.sort_by(|a, b| -> Ordering {
            let a = a.get_area();
            let b = b.get_area();
            if a > b {
                Ordering::Less
            } else if a < b {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        });

        let win_id = conn.generate_id().expect("Failed to generate window id.");

        Overlay {
            conn,
            screen,
            zones,
            atoms,
            colors: None,
            config,
            win_id,
            active_zone: None,
            pixmap: None,
        }
    }

    pub fn init(mut self) -> Result<Self, ReplyOrIdError> {
        let win_aux = CreateWindowAux::new()
            .event_mask(EventMask::EXPOSURE | EventMask::STRUCTURE_NOTIFY)
            .background_pixel(self.screen.white_pixel)
            .override_redirect(1);
        let opacity: u32 = (self.config.alpha * u32::MAX as f32) as u32;
        let wm_normal_hints = [
            15,                                  // Flags: PMinSize | PMaxSize
            self.screen.width_in_pixels as u32,  // min width
            self.screen.height_in_pixels as u32, // min height
            self.screen.width_in_pixels as u32,  // max width
            self.screen.height_in_pixels as u32, // max height
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
            0,
            0,
            self.screen.width_in_pixels,
            self.screen.height_in_pixels,
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
            self.screen.width_in_pixels,
            self.screen.height_in_pixels,
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
            let ctrl = self.button_pressed(KeyButMask::CONTROL).unwrap_or(false);
            match event {
                Event::ConfigureNotify(e) => {
                    if ctrl {
                        if !is_showing {
                            is_showing = true;
                            self.show()?;
                        }
                        win = Some(e.window);
                        let pointer = self.conn.query_pointer(self.win_id)?.reply()?;
                        self.find_active_zone(pointer.root_x, pointer.root_y);
                    }
                }
                Event::XinputRawKeyRelease(e) => {
                    if e.detail == 37 && is_showing {
                        is_showing = false;
                        self.hide()?;
                    }
                }
                Event::XinputRawButtonRelease(e) => {
                    if e.detail == 1 {
                        if is_showing {
                            if ctrl {
                                if let Some(active_win) = win {
                                    self.snap_to_zone(active_win)?;
                                    win = None;
                                } else {
                                    println!("Got no win outer");
                                }
                            }
                            is_showing = false;
                            self.hide()?;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn button_pressed(&self, but: KeyButMask) -> Result<bool, ReplyOrIdError> {
        if let Ok(reply) = self.conn.query_pointer(self.screen.root)?.reply() {
            Ok(reply.mask & but != KeyButMask::from(0_u16))
        } else {
            Ok(false)
        }
    }

    fn show(&self) -> Result<(), ReplyOrIdError> {
        self.conn.map_window(self.win_id)?;
        self.conn.configure_window(
            self.win_id,
            &ConfigureWindowAux {
                x: Some(0),
                y: Some(0),
                width: None,
                height: None,
                border_width: None,
                sibling: None,
                stack_mode: None,
            },
        )?;
        self.conn.flush()?;
        Ok(())
    }

    fn hide(&self) -> Result<(), ReplyOrIdError> {
        self.conn.unmap_window(self.win_id)?;
        self.conn.flush()?;
        Ok(())
    }

    fn snap_to_zone(&mut self, win: u32) -> Result<(), ReplyOrIdError> {
        if let Some(zone) = self.active_zone {
            let zone = &self.zones[zone];
            let conf = ConfigureWindowAux::new()
                .x(i32::from(zone.x))
                .y(i32::from(zone.y))
                .width(u32::try_from(zone.width).unwrap())
                .height(u32::try_from(zone.height).unwrap())
                .stack_mode(StackMode::ABOVE);

            self.disable_window_padding(win)?;

            self.conn.change_window_attributes(
                win,
                &ChangeWindowAttributesAux::new().win_gravity(Gravity::NORTH_WEST),
            )?;
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

    fn find_active_zone(&mut self, x: i16, y: i16) {
        let mut dist_sqr_min = u32::MAX;
        let mut zone_area_min = u32::MAX;

        for i in 0..self.zones.len() {
            if self.zones[i].is_inside(x, y) {
                let dist_sqr = self.zones[i].get_sqr_dist_to(x, y);
                let zone_area = self.zones[i].get_area();
                if dist_sqr < dist_sqr_min || dist_sqr == dist_sqr_min && zone_area < zone_area_min
                {
                    self.active_zone = Some(i);
                    dist_sqr_min = dist_sqr;
                    zone_area_min = zone_area;
                }
            }
        }

        self.draw_zones(
            self.win_id,
            self.colors.as_ref().unwrap().white.gcontext(),
            self.colors.as_ref().unwrap().black.gcontext(),
        )
        .unwrap();
    }

    fn draw_zones(&self, win_id: Window, c1: Gcontext, c2: Gcontext) -> Result<(), ReplyOrIdError> {
        for zone in &self.zones {
            let top = Rectangle {
                x: zone.x,
                y: zone.y,
                width: zone.width as u16,
                height: self.config.line_thickness,
            };

            let left = Rectangle {
                x: zone.x,
                y: zone.y,
                width: self.config.line_thickness,
                height: zone.height as u16,
            };

            let right = Rectangle {
                x: zone.x + zone.width - self.config.line_thickness as i16,
                y: zone.y,
                width: self.config.line_thickness,
                height: zone.height as u16,
            };

            let bottom = Rectangle {
                x: zone.x,
                y: zone.y + zone.height - self.config.line_thickness as i16,
                width: zone.width as u16,
                height: self.config.line_thickness,
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
}
