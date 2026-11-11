// Soft Landing's captured local player: the window's Game writes its own
// player's entity id every tick, the server thread — whose fall handling
// asks the mod — reads it. The id and its presence are separate atomics, so
// entity id zero is a real player like any other and a cleared capture
// cancels nothing.
//
// The behavior tests at the bottom drive real falls over prepared 26.3
// resources (`HORIZON_TEST_RESOURCES`, as the engine's own motion tests do):
// with the capture held the local player's id lands unhurt while a guest id
// in the same world is hurt, and with the capture cleared the local player is
// hurt again.
//
// Copyright (c) 2026 hakergeniusz. MIT.
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Whether a capture is held at all (an entity id of zero is a real id).
static PRESENT: AtomicBool = AtomicBool::new(false);
/// The captured local player's entity id.
static CAPTURED: AtomicU32 = AtomicU32::new(0);

/// The window's tick: remember the local player's entity id.
pub fn capture(id: u32) {
    CAPTURED.store(id, Ordering::Release);
    PRESENT.store(true, Ordering::Release);
}

/// Forget the capture: the window has no player to protect (no world yet, or
/// the left one), so no fall is canceled.
pub fn forget() {
    PRESENT.store(false, Ordering::Release);
}

/// The captured local player's entity id, if one is held.
pub fn captured() -> Option<u32> {
    if PRESENT.load(Ordering::Acquire) {
        Some(CAPTURED.load(Ordering::Acquire))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{capture, captured, forget};
    use crate::{
        assets::Assets,
        collisions,
        motion::{MotionEvent, MotionResources, MotionWorld, PlayerMotion},
        world::World,
    };
    use horizon_core::geometry::Vec3;
    use std::path::{Path, PathBuf};

    /// The ids the scenarios use: the window's player and a LAN guest in the
    /// same world. The engine starts every simulated player at id 0, so the
    /// scenarios give both sides real, distinct ids.
    const LOCAL: u32 = 7;
    const GUEST: u32 = 9;
    /// Full health: an unhurt landing.
    const FULL: f32 = 20.0;

    /// The prepared resources the engine's own motion tests read, or none:
    /// the scenarios then cannot run.
    fn prepared_root() -> Option<PathBuf> {
        let root = std::env::var_os("HORIZON_TEST_RESOURCES")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../.local/horizon-client")
                    .join(horizon_core::VANILLA_VERSION)
            });
        if root.join("movement-properties.nbt").exists() {
            Some(root)
        } else {
            None
        }
    }

    #[test]
    fn the_capture_holds_an_id_and_clears_whole() {
        assert_eq!(captured(), None);
        capture(LOCAL);
        assert_eq!(captured(), Some(LOCAL));
        // Entity id zero is a real id, not "no one".
        capture(0);
        assert_eq!(captured(), Some(0));
        forget();
        assert_eq!(captured(), None);
    }

    /// With the capture held, the local player's own fall is canceled: they
    /// land on the block unhurt, no damage event fires, and the fall-distance
    /// statistic — recorded upstream of the mod hook — still does.
    #[test]
    fn a_captured_local_players_fall_lands_unhurt() {
        let Some(root) = prepared_root() else {
            eprintln!("skipped soft landing regression: prepared resources unavailable");
            return;
        };
        let assets = Assets::load(&root).unwrap();
        let render = assets.reload(&root).unwrap();
        let binding = collisions::load(&root, assets.registry).unwrap();
        let resources = MotionResources::load(&root, &assets).unwrap();
        capture(LOCAL);
        let mut scene = World::test_scene(&assets, &render, 2).unwrap();
        let mut player = PlayerMotion::new(&resources, Vec3::new(20.5, 20.0, 20.5)).unwrap();
        player.entity_id = LOCAL as i32;
        player.fall_distance = 30.0;
        let mut w = MotionWorld {
            entities: &[],
            player: &mut player,
            scene: &mut scene,
            assets: &assets,
            collision_binding: &binding,
            resources: &resources,
            health: &resources.health,
        };
        w.move_self(Vec3::new(0.0, -30.0, 0.0)).unwrap();
        assert!(w.player.ground.on_ground);
        assert_eq!(w.player.health.health(), FULL);
        assert!(w.player.events.iter().any(|e| matches!(
            e,
            MotionEvent::Fall(crate::landing::FallEvent::Statistic(_))
        )));
        assert!(!w.player.events.iter().any(|e| matches!(
            e,
            MotionEvent::Fall(crate::landing::FallEvent::Damage { .. })
        )));
        forget();
    }

    /// In the same state, a guest's fall in the same world keeps the vanilla
    /// damage: the capture is the local player's id, not theirs.
    #[test]
    fn a_guest_in_the_same_world_is_still_hurt() {
        let Some(root) = prepared_root() else {
            eprintln!("skipped soft landing regression: prepared resources unavailable");
            return;
        };
        let assets = Assets::load(&root).unwrap();
        let render = assets.reload(&root).unwrap();
        let binding = collisions::load(&root, assets.registry).unwrap();
        let resources = MotionResources::load(&root, &assets).unwrap();
        capture(LOCAL);
        let mut scene = World::test_scene(&assets, &render, 2).unwrap();
        let mut player = PlayerMotion::new(&resources, Vec3::new(20.5, 20.0, 20.5)).unwrap();
        player.entity_id = GUEST as i32;
        player.fall_distance = 30.0;
        let mut w = MotionWorld {
            entities: &[],
            player: &mut player,
            scene: &mut scene,
            assets: &assets,
            collision_binding: &binding,
            resources: &resources,
            health: &resources.health,
        };
        w.move_self(Vec3::new(0.0, -30.0, 0.0)).unwrap();
        assert!(w.player.ground.on_ground);
        assert!(w.player.health.health() < FULL);
        forget();
    }

    /// With the capture cleared, the local player is hurt again: the mod is
    /// the only thing standing between them and the vanilla fall damage.
    #[test]
    fn a_cleared_capture_leaves_the_local_player_vanilla() {
        let Some(root) = prepared_root() else {
            eprintln!("skipped soft landing regression: prepared resources unavailable");
            return;
        };
        let assets = Assets::load(&root).unwrap();
        let render = assets.reload(&root).unwrap();
        let binding = collisions::load(&root, assets.registry).unwrap();
        let resources = MotionResources::load(&root, &assets).unwrap();
        capture(LOCAL);
        forget();
        let mut scene = World::test_scene(&assets, &render, 2).unwrap();
        let mut player = PlayerMotion::new(&resources, Vec3::new(20.5, 20.0, 20.5)).unwrap();
        player.entity_id = LOCAL as i32;
        player.fall_distance = 30.0;
        let mut w = MotionWorld {
            entities: &[],
            player: &mut player,
            scene: &mut scene,
            assets: &assets,
            collision_binding: &binding,
            resources: &resources,
            health: &resources.health,
        };
        w.move_self(Vec3::new(0.0, -30.0, 0.0)).unwrap();
        assert!(w.player.ground.on_ground);
        assert!(w.player.health.health() < FULL);
    }
}
