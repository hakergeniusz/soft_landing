//! Soft Landing: fall damage never lands. The `living_fall` hook fires
//! inside the tick's own landing, before damage and block effects, wherever
//! this machine ticks the falling entity - the local single-player world and
//! every player (the host and its guests) on a world this machine hosts.

use mod_api::{Flow, Hooks};

#[derive(Default)]
pub struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// A living entity is about to handle a fall: cancel the handling, so
    /// the fall deals no damage and triggers no landing effects.
    fn living_fall(
        &self,
        context: &dyn mod_api::LoopContext,
        event: &mut mod_api::LivingFallEvent<'_>,
    ) -> Flow {
        let _ = context;
        event.cancelled = true;
        Flow::Continue
    }
}
