use std::error::Error;

use x11rb::COPY_DEPTH_FROM_PARENT;
use x11rb::connection::{Connection, RequestConnection as _};
use x11rb::errors::{ConnectionError, ReplyOrIdError};
use x11rb::protocol::Event;
use x11rb::protocol::shape::{self, ConnectionExt as _, SK};
use x11rb::protocol::xfixes::{self, CreateRegionRequest, create_region, set_window_shape_region};
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

use crate::config::Zone;

pub struct Overlay<'a, C: Connection + 'a> {
    conn: &'a C,
    screen: &'a Screen,
    window_size: (u16, u16),
    has_shape: bool,
    pixmap: PixmapWrapper<&'a C>,
    win_id: Window,
    zones: Vec<Zone>,
    wm_protocols: u32,
    wm_delete_window: u32,
    net_wm_state: u32,
    net_wm_state_above: u32,
    white_gc: GcontextWrapper<&'a C>,
    black_gc: GcontextWrapper<&'a C>,
}

impl<'a> Overlay<'a, RustConnection> {
    pub fn new(
        conn: &'a RustConnection,
        screen_num: usize,
        zones: Vec<Zone>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let screen = &conn.setup().roots[screen_num];

        let wm_protocols = conn.intern_atom(false, b"WM_PROTOCOLS")?.reply()?.atom;
        let wm_delete_window = conn.intern_atom(false, b"WM_DELETE_WINDOW")?.reply()?.atom;

        let net_wm_state = conn.intern_atom(false, b"_NET_WM_STATE")?.reply()?.atom;
        let net_wm_state_above = conn
            .intern_atom(false, b"_NET_WM_STATE_ABOVE")?
            .reply()?
            .atom;

        let window_size = (screen.width_in_pixels, screen.height_in_pixels);

        let alpha = 0.5_f32;

        let has_shape = conn
            .extension_information(shape::X11_EXTENSION_NAME)
            .expect("failed to get extension information")
            .is_some();

        let win_id = setup_window(
            &conn,
            screen,
            window_size,
            wm_protocols,
            wm_delete_window,
            alpha,
        )?;
        let pixmap = PixmapWrapper::create_pixmap(
            conn,
            screen.root_depth,
            win_id,
            window_size.0,
            window_size.1,
        )?;

        let white_gc = create_gc_with_foreground(conn, win_id, screen.white_pixel)?;
        let black_gc = create_gc_with_foreground(conn, win_id, screen.black_pixel)?;

        Ok(Overlay {
            conn: &conn,
            screen,
            window_size,
            has_shape,
            pixmap,
            win_id,
            zones,
            wm_delete_window,
            wm_protocols,
            net_wm_state_above,
            net_wm_state,
            white_gc,
            black_gc,
        })
    }

    pub fn run_until(&mut self, condition: fn() -> bool) -> Result<(), Box<dyn std::error::Error>> {
        self.conn.flush()?;

        let mut need_repaint = false;
        let mut need_reshape = false;
        let mut active_zone: Option<&Zone> = None;

        while condition() {
            let event = self.conn.wait_for_event()?;
            let mut event_option = Some(event);
            while let Some(event) = event_option {
                match event {
                    Event::Expose(event) => {
                        if event.count == 0 {
                            need_repaint = true;
                        }
                    }
                    Event::ConfigureNotify(event) => {
                        self.window_size = (event.width, event.height);
                        self.pixmap = PixmapWrapper::create_pixmap(
                            self.conn,
                            self.screen.root_depth,
                            self.win_id,
                            self.window_size.0,
                            self.window_size.1,
                        )?;
                        need_reshape = true;
                    }
                    Event::MotionNotify(event) => {
                        active_zone = find_active_zone(event.event_x, event.event_y, &self.zones);
                        need_reshape = true;
                        need_repaint = true;
                    }
                    Event::MapNotify(_) => {
                        set_always_on_top(
                            self.conn,
                            self.screen.root,
                            self.win_id,
                            self.net_wm_state,
                            self.net_wm_state_above,
                        )
                        .unwrap();
                        need_reshape = true;
                    }
                    Event::ClientMessage(event) => {
                        let data = event.data.as_data32();
                        if event.format == 32
                            && event.window == self.win_id
                            && data[0] == self.wm_delete_window
                        {
                            println!("Window was asked to close");
                            return Ok(());
                        }
                    }
                    Event::Error(error) => {
                        println!("Unknown error {:?}", error);
                    }
                    event => {
                        println!("Unknown event {:?}", event);
                    }
                }

                event_option = self.conn.poll_for_event()?;
            }

            if need_reshape && self.has_shape {
                shape_window(
                    self.conn,
                    self.win_id,
                    self.window_size,
                    &self.zones,
                    active_zone,
                )?;
                need_reshape = false;
            }
            if need_repaint {
                draw_zones(
                    self.conn,
                    self.pixmap.pixmap(),
                    self.white_gc.gcontext(),
                    self.black_gc.gcontext(),
                    &self.zones,
                    active_zone,
                )?;

                self.conn.copy_area(
                    self.pixmap.pixmap(),
                    self.win_id,
                    self.white_gc.gcontext(),
                    0,
                    0,
                    0,
                    0,
                    self.window_size.0,
                    self.window_size.1,
                )?;

                self.conn.flush()?;
                need_repaint = false;
            }
        }
        Ok(())
    }
}

fn draw_zones<C: Connection>(
    conn: &C,
    win_id: Window,
    white_gc: Gcontext,
    black_gc: Gcontext,
    zones: &[Zone],
    active_zone: Option<&Zone>,
) -> Result<(), ConnectionError> {
    for zone in zones {
        // let xy = Point {
        //     x: zone.x,
        //     y: zone.y,
        // };
        // let xwy = Point {
        //     x: zone.x + zone.width,
        //     y: zone.y,
        // };
        // let xyw = Point {
        //     x: zone.x,
        //     y: zone.y + zone.height,
        // };
        // let xwyw = Point {
        //     x: zone.x + zone.width,
        //     y: zone.y + zone.height,
        // };
        for i in 0..10_i16 {
            let rect = Rectangle {
                x: zone.x + i,
                y: zone.y + i,
                width: (zone.width - i) as u16,
                height: (zone.height - i) as u16,
            };
            conn.poly_rectangle(win_id, black_gc, &[rect])?;
        }
        // conn.poly_line(
        //     CoordMode::ORIGIN,
        //     win_id,
        //     white_gc,
        //     &[xy, xwy, xwyw, xyw, xy],
        // )?;
    }
    draw_active_zone(conn, win_id, white_gc, active_zone)?;
    Ok(())
}

fn draw_active_zone<C: Connection>(
    conn: &C,
    win_id: Window,
    gc: Gcontext,
    zone: Option<&Zone>,
) -> Result<(), ConnectionError> {
    if zone.is_some() {
        let zone = zone.unwrap();
        let rect = Rectangle {
            x: zone.x,
            y: zone.y,
            width: zone.width as u16,
            height: zone.height as u16,
        };

        conn.poly_fill_rectangle(win_id, gc, &[rect])?;
    }
    Ok(())
}

fn shape_window<C: Connection>(
    conn: &C,
    win_id: Window,
    window_size: (u16, u16),
    zones: &[Zone],
    active_zone: Option<&Zone>,
) -> Result<(), ReplyOrIdError> {
    // Create a pixmap for the shape
    let pixmap = PixmapWrapper::create_pixmap(conn, 1, win_id, window_size.0, window_size.1)?;

    // Fill the pixmap with what will indicate "transparent"
    let gc = create_gc_with_foreground(conn, pixmap.pixmap(), 0)?;

    let rect = Rectangle {
        x: 0,
        y: 0,
        width: window_size.0,
        height: window_size.1,
    };
    conn.poly_fill_rectangle(pixmap.pixmap(), gc.gcontext(), &[rect])?;

    let values = ChangeGCAux::new().foreground(1);
    conn.change_gc(gc.gcontext(), &values)?;
    draw_zones(
        conn,
        pixmap.pixmap(),
        gc.gcontext(),
        gc.gcontext(),
        zones,
        active_zone,
    )?;

    // Set the shape of the window
    conn.shape_mask(shape::SO::SET, shape::SK::BOUNDING, win_id, 0, 0, &pixmap)?;
    Ok(())
}

fn setup_window<C: Connection>(
    conn: &C,
    screen: &Screen,
    window_size: (u16, u16),
    wm_protocols: Atom,
    wm_delete_window: Atom,
    alpha: f32,
) -> Result<Window, ReplyOrIdError> {
    let win_id = conn.generate_id()?;
    let win_aux = CreateWindowAux::new()
        .event_mask(EventMask::EXPOSURE | EventMask::STRUCTURE_NOTIFY | EventMask::POINTER_MOTION)
        .background_pixel(screen.white_pixel);

    let motif_wm_hints = conn.intern_atom(false, b"_MOTIF_WM_HINTS")?.reply()?.atom;
    let no_decorations_hint: [u32; 5] = [2, 0, 0, 0, 0];

    let alpha = alpha.clamp(0.0, 1.0);
    let opacity: u32 = (alpha * u32::MAX as f32) as u32;
    let wm_window_opacity = conn
        .intern_atom(false, b"_NET_WM_WINDOW_OPACITY")?
        .reply()?
        .atom;

    let wm_normal_hints = [
        15,                   // Flags: PMinSize | PMaxSize
        window_size.0 as u32, // min width
        window_size.1 as u32, // min height
        window_size.0 as u32, // max width
        window_size.1 as u32, // max height
        0,
        0,
        0,
        0,
        0,
    ];

    conn.create_window(
        COPY_DEPTH_FROM_PARENT,
        win_id,
        screen.root,
        0,
        0,
        window_size.0,
        window_size.1,
        0,
        WindowClass::INPUT_OUTPUT,
        0,
        &win_aux,
    )?;

    let title = "lancy-zones";
    conn.change_property8(
        PropMode::REPLACE,
        win_id,
        AtomEnum::WM_NAME,
        AtomEnum::STRING,
        title.as_bytes(),
    )?;
    conn.change_property32(
        PropMode::REPLACE,
        win_id,
        wm_protocols,
        AtomEnum::ATOM,
        &[wm_delete_window],
    )?;
    conn.change_property32(
        PropMode::REPLACE,
        win_id,
        motif_wm_hints,
        AtomEnum::ATOM,
        &no_decorations_hint,
    )?;
    conn.change_property32(
        PropMode::REPLACE,
        win_id,
        wm_window_opacity,
        AtomEnum::CARDINAL,
        &[opacity],
    )?;
    conn.change_property32(
        PropMode::REPLACE,
        win_id,
        AtomEnum::WM_NORMAL_HINTS,
        AtomEnum::CARDINAL,
        &wm_normal_hints,
    )?;

    conn.map_window(win_id)?;
    conn.flush()?;

    Ok(win_id)
}

fn create_gc_with_foreground<C: Connection>(
    conn: C,
    win_id: Window,
    foreground: u32,
) -> Result<GcontextWrapper<C>, ReplyOrIdError> {
    GcontextWrapper::create_gc(
        conn,
        win_id,
        &CreateGCAux::new()
            .graphics_exposures(0)
            .foreground(foreground),
    )
}

fn set_always_on_top<C: Connection>(
    conn: &C,
    root: Window,
    win_id: Window,
    net_wm_state: Atom,
    net_wm_state_above: Atom,
) -> Result<(), Box<dyn Error>> {
    let data = ClientMessageData::from([1, net_wm_state_above, 0, 0, 0]); // 1 = _NET_WM_STATE_ADD

    let msg = ClientMessageEvent {
        response_type: CLIENT_MESSAGE_EVENT,
        format: 32,
        window: win_id,
        type_: net_wm_state,
        data,
        sequence: 0,
    };

    conn.send_event(
        false,
        root,
        EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
        msg,
    )?;

    conn.flush()?;
    Ok(())
}

fn find_active_zone(x: i16, y: i16, zones: &[Zone]) -> Option<&Zone> {
    for zone in zones {
        if x >= zone.x && x <= zone.x + zone.width && y >= zone.y && y <= zone.y + zone.height {
            return Some(zone);
        }
    }
    None
}
