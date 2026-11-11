# Soft Landing

The local player never takes fall damage, from any height. A Horizon native
mod (mixin bundle), always on, with no settings and no UI — and everyone else
keeps the vanilla fall damage.

Clean-room: implemented from a behavior spec against Horizon's own mod API
(the game's `native_living_fall` hook, its `LivingFallEvent` dispatch). No
other mod's code, text or assets was consulted or copied.

## What you get

- **No fall damage for you** — any height, every landing. The game's fall
  handling is canceled for your falls: nothing hurts you.
- **You still land** — the block's own landing reaction runs as vanilla's:
  hay cushions, slime and honey bounce, farmland can trample, turtle eggs can
  break, and landing sounds play.
- **The fall-distance statistic still records** — the game counts your fall
  distance upstream of where the mod decides, so the statistic entry fires
  for canceled falls too. That is expected and kept.
- **Nobody else is affected** — mobs, LAN guests and other players keep the
  vanilla fall damage. Only the local player's own falls are canceled.

## Scope limits

Soft Landing is client-side and only controls worlds **simulated inside this
process**: singleplayer worlds (the integrated server) and LAN worlds you
host. On a **dedicated server** fall damage is computed server-side; a client
mod cannot change it there. There is no settings UI in 1.0.0 — install it and
it is always active.

## How it fits together

The window's `Game` writes its own player's entity id every tick; the
server thread — the one that handles the falls — reads it. The mod sits at
the head of the game's own fall hook (`native_living_fall`) and cancels the
fall when the falling entity is that captured id and the player kind, so the
caller lands the player with no damage and still applies the block's `fallOn`
reaction. The decision is three lines of plain Rust (pure standard library,
doubled as the crate's library and tested); the helpers are thin splices.

Verification recipe:

```sh
python3 -I tools/test.py           # fmt, clippy, the policy's tests
python3 -I tools/horizon.py check  # staged cargo check on the pinned revision
python3 -I tools/horizon.py test   # staged real-fall behavior tests
python3 -I tools/horizon.py build  # staged fast build
python3 -I tools/package.py        # target/soft_landing-1.0.0.hnmod
```

The staged tests drive real falls through the game's own movement
(`MotionWorld::move_self`) over prepared 26.3 resources: with the capture
held the local player's id lands unhurt, a guest id in the same world is
hurt, and with the capture cleared the local player is hurt again.

## License

MIT — see [LICENSE](LICENSE).
