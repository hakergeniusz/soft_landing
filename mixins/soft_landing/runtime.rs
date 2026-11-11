// Soft Landing's brain: pure standard library, no game types, so it compiles
// and tests wherever — as this crate's library and, injected, as a module of
// the game's fall-handling file. This is the whole policy: which falls lose
// their damage. The injected helpers feed it the captured local player's id
// and carry its answer into the game's fall handling.
//
// Clean-room: implemented from a behavior spec against Horizon's own mod API;
// no other mod's code, text or assets was consulted or copied.
//
// Copyright (c) 2026 hakergeniusz. MIT.

/// The entity kind the game's fall handling reports for a player, both for
/// the local player's own fall and for everyone else's.
pub const PLAYER_KIND: &str = "minecraft:player";

/// Whether a fall should be canceled: only the local player's own fall —
/// reported as the player kind, carrying the id the window's tick captured —
/// is canceled. Everyone else keeps the vanilla fall damage: mobs, LAN
/// guests, other players. A cleared capture (the window has no world, or the
/// tests asked for vanilla back) cancels nothing, and an entity id of zero is
/// a real id like any other, so the capture carries its presence separately
/// from the id itself.
pub fn cancels(kind: &str, entity: u32, local: Option<u32>) -> bool {
    kind == PLAYER_KIND && local == Some(entity)
}

#[cfg(test)]
mod tests {
    use super::{PLAYER_KIND, cancels};

    const LOCAL: u32 = 7;
    const GUEST: u32 = 9;

    #[test]
    fn a_captured_local_players_fall_is_canceled() {
        assert!(cancels(PLAYER_KIND, LOCAL, Some(LOCAL)));
        // Entity id zero is a real id, not "no one".
        assert!(cancels(PLAYER_KIND, 0, Some(0)));
    }

    #[test]
    fn anyone_elses_fall_keeps_the_vanilla_damage() {
        // A guest or mob id in the same world.
        assert!(!cancels(PLAYER_KIND, GUEST, Some(LOCAL)));
        // A mob's kind, even with a matching id.
        assert!(!cancels("minecraft:zombie", LOCAL, Some(LOCAL)));
    }

    #[test]
    fn a_cleared_capture_cancels_nothing() {
        assert!(!cancels(PLAYER_KIND, LOCAL, None));
        // And the capture's presence is separate from the id, so a real
        // player with id 0 is protected while the capture is held and
        // vanilla once it is cleared.
        assert!(cancels(PLAYER_KIND, 0, Some(0)));
        assert!(!cancels(PLAYER_KIND, 0, None));
    }
}
