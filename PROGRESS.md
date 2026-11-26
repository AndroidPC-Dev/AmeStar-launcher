# AmeStar progress (branch dev)

Last completed file: mobile_controls/mod.rs (in-game Customize Controls opens Amethyst's CustomControlsActivity; game loads controlmap/default.json)

Next exact task: user tests; watch CustomControlsActivity start from the game process, layout import (pojav.rs), then trim Starling menus.

Notes
- App = Amethyst-Android (amethyst/) + Starling engine staged by packaging/android/stage_gradle.py. Package io.github.animexdevil.amestar, arm64 only.
- Files: Android/data/<pkg>/files (.minecraft, resources, Starling).
- Mapped settings: general, display, quality, performance, hdr, extras, mods, touch (see amestar_settings.rs). Not mapped yet: enums (upscaling, api, present mode, shader pack), orientation/aspect/safe areas, skin.
- Starling-only Rust UI (crates/horizon-client/src/ha_launcher) is no longer the launcher; old workflow amestar-android is manual-only.
- Project rule: work on dev, one file per commit, never push main.
