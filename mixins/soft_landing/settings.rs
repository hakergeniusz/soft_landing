// The mod's settings page: the guest-protection toggle, then a live note
// saying when the protection is actually acting on a hosted world — the
// page always shows what the fall gate acts on right now.
// Copyright (c) 2026 hakergeniusz. MIT.
fn soft_landing_page(
    game: &mut crate::game::Game<'_>,
    mod_id: &str,
    buttons: &mut Vec<String>,
) {
    use crate::landing::gate;
    if mod_id != "soft_landing" {
        return;
    }
    let _ = game;
    buttons.push(format!(
        "Protect guests: {}",
        if gate::protects_guests() { "ON" } else { "OFF" }
    ));
    buttons.push(if gate::is_hosting() {
        "This world is open to the network: every player on it lands unhurt.".to_string()
    } else {
        "Guests are shielded when you open a world to LAN.".to_string()
    });
}
