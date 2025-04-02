fn main() {
    let (conn, screen_num) = x11rb::connect(None).unwrap();
    let screen = conn.setup().roots[screen_num].clone();
    conn.change_property32(
        PropMode::REPLACE,
        win_id,
        atoms.motif_wm_hints,
        AtomEnum::ATOM,
        &self.atoms.no_decorations_hint,
    )?;
}
