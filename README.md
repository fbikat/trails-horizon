# Trails

Client-side paw prints, particles, lines and ribbons that follow you around, for
[Horizon](https://playhorizon.dev). Only you see them; nothing is needed on the server.

## Modes

- **Cat paws**: alternating paw prints on the blocks you walk on. They follow the local light and disappear when
  the block under them is removed or changes height.
- **Particle**: sparkles, dust, orbs or hearts behind you.
- **Line**: a thin line along your path.
- **Ribbon**: a flat ribbon along your path.

Trails fade after a few seconds, or stay (bounded in memory) in persistent mode. Changing mode, dimension or world
clears them.

## Controls

- **K** turns trails on and off (rebind it in Options > Mods > Trails).
- **Options > Mods > Trails > Settings** shows the settings for the current mode. Clicking a button cycles it:
  numbers step up and wrap, colours cycle through a palette.
- Chat commands:
  - `/trails on|off|toggle`
  - `/trails mode paws|particle|line|ribbon`
  - `/trails set <setting> <value>`, e.g. `/trails set color #7FD4FF`, `/trails set pawSize 0.5`
  - `/trails settings` lists every setting and its value
  - `/trails clear`, `/trails reset`

Settings are kept in `horizon-mods/.horizon-mod-data/trails/`, one file per setting.

## Install

From the Horizon launcher's mod browser, or from source:

```
horizon-launcher native-build mixins/trails --trust-native-code
```

## Layout

| Path | What it is |
|---|---|
| `mixins/trails/horizon-mixin.toml` | The bundle: one module and nine HEAD hooks |
| `mixins/trails/runtime.rs` | All the mod's logic and state, added to the game as `game::trails_mod`; also this crate's library, for tests |
| `mixins/trails/*.rs` | One small helper per hook (tick, frame, world draw, lines, key, command, settings) |
| `mixins/trails/data/atlas.rgba` | The textures as raw RGBA, compiled in with `native_data!` |
| `textures/` | The source PNGs of the atlas |
| `tools/pack_atlas.py` | Packs `textures/` into `atlas.rgba` |
| `tools/package.py` | Writes `dist/trails-<version>.hnmod` |

## Development

```
cargo test
cargo clippy --all-targets -- -D warnings
python tools/pack_atlas.py textures mixins/trails/data/atlas.rgba
python tools/package.py .
```

The bundle pins Horizon Nightly 2026-11-07 (`2ebf00a`, hook points API 411): the world draw hooks
(`native_world_submit`, `native_world_primitives`) aren't in the named hook catalog yet, so it targets them by path.
It passes horizon-mods' `native.py check` on that commit.

`tools/package.py` is used instead of horizon-mods' packager because that one leaves out `mixins/<id>/data/`, where
the textures are.

## Differences from the Fabric version

- No slider screen: settings are buttons on Horizon's mod page, plus `/trails set` for exact values and custom
  colours. K toggles trails instead of opening a screen.
- Standing on the ground is read from block collision under your feet, so paws can appear while flying just above
  the ground.
- Lines use Horizon's line primitives, which aren't lightmapped; they are darkened by the local light level instead.
- Ribbons use one colour per segment.
- The textures use Horizon's shared mod image slot (`horizon/modimage`); another mod uploading to it will clash.

## Licence

MIT, see [LICENSE](LICENSE).
