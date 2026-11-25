# AmeStar progress (branch dev)

Last completed file: amethyst/.../StarlingSettingsFragment.java (settings file now AmeStar/Starling/starling-settings.json, read by the engine via crates/horizon-client-render/src/amestar_settings.rs)

Next exact task: run workflow amethyst-android on ref dev with light=false, engine=true (final full APK); read the log; fix any error; then check on phone: settings pages, controls editor, PLAY -> Starling.

Notes
- App = Amethyst-Android (amethyst/) + Starling engine staged by packaging/android/stage_gradle.py. Package io.github.animexdevil.amestar, arm64 only.
- Files: /storage/emulated/0/AmeStar (.minecraft = official files, Starling/resources = prepared, Starling/starling-settings.json).
- Mapped settings: general, display, quality, performance, hdr, extras, mods, touch (see amestar_settings.rs). Not mapped yet: enums (upscaling, api, present mode, shader pack), orientation/aspect/safe areas, skin.
- Starling-only Rust UI (crates/horizon-client/src/ha_launcher) is no longer the launcher; old workflow amestar-android is manual-only.
- Project rule: work on dev, one file per commit, never push main.
