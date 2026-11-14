// The mod's row in Options > Mods: the mod's identity and what it does. No
// keybinds — the one toggle lives on the settings page.
// Copyright (c) 2026 hakergeniusz. MIT.
fn soft_landing_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    mods.push(crate::game::native_hooks::NativeMod {
        id: "soft_landing".into(),
        title: "Soft Landing 1.1.0".into(),
        description: "No fall damage for you — and for every player on a world you open to LAN (toggle on the settings page).".into(),
        keys: vec![],
    });
}
