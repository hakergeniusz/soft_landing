// Soft Landing's capture: the head of the tick every Game runs. Only the
// window's Game writes (singleplayer worlds and LAN hosts play there); the
// server's own Game — the thread that handles the falls — never does, so it
// reads what the window wrote. A window without a world has no player to
// protect and clears the capture.
//
// Copyright (c) 2026 hakergeniusz. MIT.
fn soft_landing_capture(game: &mut crate::game::Game<'_>) {
    if !game.native_window() {
        return;
    }
    match game.remote.as_ref().map(|r| r.entity_id as u32) {
        Some(id) => crate::landing::gate::capture(id),
        None => crate::landing::gate::forget(),
    }
}
