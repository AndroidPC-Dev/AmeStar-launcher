# AmeStar progress (branch dev)

Last completed file: amethyst/.../MainMenuFragment.java (home: no wiki/discord/profile bar; engine: no in-game options menu; resources copy across filesystems)

Next exact task: user runs amestar-app on dev (engine on, light off); check on phone: PLAY starts game after resources, settings pages, controls editor.

Notes
- App = Amethyst-Android (amethyst/) + Starling engine staged by packaging/android/stage_gradle.py. Package io.github.animexdevil.amestar, arm64 only.
- Files: /storage/emulated/0/AmeStar (.minecraft = official files, Starling/resources = prepared, Starling/starling-settings.json).
- Mapped settings: general, display, quality, performance, hdr, extras, mods, touch (see amestar_settings.rs). Not mapped yet: enums (upscaling, api, present mode, shader pack), orientation/aspect/safe areas, skin.
- Starling-only Rust UI (crates/horizon-client/src/ha_launcher) is no longer the launcher; old workflow amestar-android is manual-only.
- Project rule: work on dev, one file per commit, never push main.
