// Soft Landing's capture: the head of the tick every Game runs. Only the
// window's Game writes (singleplayer worlds and LAN hosts play there); the
// server's own Game — the thread that handles the falls — never does, so it
// reads what the window wrote. A window without a world has no player to
// protect and clears the capture. The window also publishes whether its
// world is open to the network (the LAN host flag the fall gate reads) and,
// once per process, syncs the saved guest-protection choice.
//
// Copyright (c) 2026 hakergeniusz. MIT.
fn soft_landing_capture(game: &mut crate::game::Game<'_>) {
    if !game.native_window() {
        return;
    }
    match crate::game::Game::native_config_dir() {
        Ok(dir) => crate::landing::gate::sync_saved(Some(&dir)),
        Err(_) => crate::landing::gate::sync_saved(None),
    }
    crate::landing::gate::hosting(game.lan.is_some());
    match game.remote.as_ref().map(|r| r.entity_id as u32) {
        Some(id) => crate::landing::gate::capture(id),
        None => crate::landing::gate::forget(),
    }
}
