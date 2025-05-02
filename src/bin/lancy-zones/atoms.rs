use x11rb::{connection::Connection, errors::ReplyOrIdError, protocol::xproto::*};

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
