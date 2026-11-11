// Soft Landing's fall gate: the head of the game's own mod hook for falls
// (`native_living_fall`, the game's LivingFallEvent dispatch). When the
// falling entity is the captured local player, the whole fall handling is
// canceled — no damage, the caller still lands the player and asks the
// block's own fall reaction — and no other hook is consulted. Everyone else
// falls on to the game's own answer.
//
// Copyright (c) 2026 hakergeniusz. MIT.
fn soft_landing_fall(
    entity: u32,
    kind: &str,
    cancel: &mut bool,
) -> core::ops::ControlFlow<()> {
    if soft_landing::cancels(kind, entity, gate::captured()) {
        *cancel = true;
        return core::ops::ControlFlow::Break(());
    }
    core::ops::ControlFlow::Continue(())
}
