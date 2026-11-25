//! The settings AmeStar's launcher part wrote (`starling-settings.json`, one plain key per value, only
//! those the player changed), put over the options the game loaded. Keys this table does not know are
//! ignored, so a newer launcher never breaks an older game.

use crate::settings::Settings;
use serde_json::Value;
use std::path::PathBuf;

/// The file's place: the game's main folder, named by `AMESTAR_ROOT` (set before the game starts).
pub fn file() -> Option<PathBuf> {
    let root = std::env::var_os("AMESTAR_ROOT")?;
    Some(PathBuf::from(root).join("starling-settings.json"))
}

/// Reads the file, if there is one, into `settings`.
pub fn apply_file(settings: &mut Settings) {
    let Some(path) = file() else { return };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return;
    };
    match serde_json::from_str::<Value>(&text) {
        Ok(json) => apply(settings, &json),
        Err(e) => log::warn!("AmeStar: {} not used: {e}", path.display()),
    }
}

fn int(json: &Value, key: &str) -> Option<i64> {
    let v = json.get(key)?;
    v.as_i64().or_else(|| v.as_f64().map(|f| f as i64))
}

fn flag(json: &Value, key: &str) -> Option<bool> {
    json.get(key)?.as_bool()
}

fn text<'a>(json: &'a Value, key: &str) -> Option<&'a str> {
    json.get(key)?.as_str()
}

/// Puts each known key of `json` into `settings`.
pub fn apply(settings: &mut Settings, json: &Value) {
    macro_rules! set {
        ($field:expr, $value:expr) => {
            if let Some(v) = $value {
                $field = v;
            }
        };
    }
    // General and display
    set!(settings.render_distance, int(json, "renderDistance").map(|v| v.clamp(2, 32) as i32));
    set!(settings.simulation_distance, int(json, "simulationDistance").map(|v| v.clamp(5, 32) as i32));
    set!(settings.horizon.lod_distance, int(json, "lodDistance").map(|v| v.clamp(0, 64) as u32));
    set!(settings.gamma, int(json, "gamma").map(|v| v.clamp(0, 100) as f32 / 100.0));
    set!(
        settings.gui_scale,
        text(json, "guiScale").map(|v| v.parse::<u32>().unwrap_or(0).min(4))
    );
    set!(settings.fullscreen, flag(json, "fullscreen"));
    set!(settings.vsync, flag(json, "vsync"));
    set!(settings.max_fps, int(json, "fpsLimit").map(|v| v.clamp(10, 260) as u32));
    set!(settings.menu_blur, int(json, "menuBlur").map(|v| v.clamp(0, 10) as i32));
    set!(
        settings.attack_indicator,
        text(json, "attackIndicator").map(|v| match v {
            "Off" => 0,
            "Hotbar" => 2,
            _ => 1,
        })
    );
    set!(settings.autosave_indicator, flag(json, "autosaveIndicator"));
    // Quality
    set!(settings.clouds, text(json, "clouds").map(|v| v.to_owned()));
    set!(settings.cloud_range, int(json, "cloudRange").map(|v| v.clamp(2, 128) as i32));
    set!(settings.horizon.sky, flag(json, "sky"));
    set!(settings.horizon.fog, flag(json, "fog"));
    set!(settings.horizon.weather, flag(json, "weather"));
    set!(settings.weather_radius, int(json, "weatherRange").map(|v| v.clamp(3, 16) as i32));
    set!(
        settings.particles,
        text(json, "particles").map(|v| match v {
            "Decreased" => 1,
            "Minimal" => 2,
            _ => 0,
        })
    );
    set!(settings.smooth_lighting, flag(json, "ambientOcclusion"));
    set!(settings.entity_distance, int(json, "entityDistance").map(|v| v.clamp(50, 500) as f32 / 100.0));
    set!(settings.entity_shadows, flag(json, "entityShadows"));
    set!(settings.vignette, flag(json, "vignette"));
    set!(settings.chunk_fade, flag(json, "chunkFade").map(|v| if v { 1.0 } else { 0.0 }));
    set!(settings.mipmap_levels, int(json, "mipmaps").map(|v| v.clamp(0, 4) as i32));
    set!(settings.cutout_leaves, text(json, "leaves").map(|v| v != "Fast"));
    set!(settings.horizon.translucent_batching, flag(json, "translucentBatching"));
    set!(settings.horizon.animated_textures, flag(json, "animatedTextures"));
    set!(settings.horizon.dither_cutout_mips, flag(json, "ditherCutoutMips"));
    set!(settings.horizon.resolution_scale, int(json, "resolutionScale").map(|v| v.clamp(25, 100) as u32));
    set!(settings.horizon.dynamic_resolution, flag(json, "dynamicResolution"));
    set!(settings.horizon.target_fps, int(json, "targetFps").map(|v| v.clamp(20, 120) as u32));
    set!(settings.horizon.fxaa, flag(json, "fxaa"));
    set!(settings.horizon.taa, flag(json, "taa"));
    set!(settings.horizon.upscaling_sharpness, int(json, "upscalingSharpness").map(|v| v.clamp(0, 100) as u32));
    set!(settings.horizon.screenshot_quality, int(json, "screenshotQuality").map(|v| v.clamp(10, 100) as u32));
    // Performance
    set!(settings.horizon.chunk_updates, int(json, "chunkUpdates").map(|v| v.clamp(1, 16) as u32));
    set!(settings.horizon.upload_budget_kib, int(json, "uploadBudget").map(|v| v.clamp(256, 16384) as u32));
    set!(settings.horizon.texture_budget_mib, int(json, "textureBudget").map(|v| v.clamp(128, 4096) as u32));
    set!(settings.horizon.workers, int(json, "workers").map(|v| v.clamp(0, 16) as u32));
    set!(settings.horizon.cave_culling, flag(json, "caveCulling"));
    set!(settings.horizon.opaque_ice, flag(json, "opaqueIce"));
    set!(settings.horizon.cache_shulker_boxes, flag(json, "cacheShulkerBoxes"));
    set!(settings.horizon.cache_signs, flag(json, "cacheSigns"));
    set!(settings.horizon.cache_decorated_pots, flag(json, "cacheDecoratedPots"));
    set!(settings.horizon.cache_bells, flag(json, "cacheBells"));
    set!(settings.horizon.cache_chests, flag(json, "cacheChests"));
    set!(settings.horizon.cache_banners, flag(json, "cacheBanners"));
    set!(settings.horizon.cache_copper_golem_statues, flag(json, "cacheCopperGolemStatues"));
    set!(settings.horizon.battery, flag(json, "battery"));
    set!(settings.horizon.shadow_cache, flag(json, "shadowCache"));
    // HDR
    set!(settings.horizon.hdr_paper_white, int(json, "hdrPaperWhite").map(|v| v.clamp(80, 500) as u32));
    set!(settings.horizon.hdr_peak, int(json, "hdrPeak").map(|v| v.clamp(400, 4000) as u32));
    set!(settings.horizon.hdr_black_level, int(json, "hdrBlackLevel").map(|v| v.clamp(0, 100) as u32));
    set!(settings.horizon.hdr_ui_nits, int(json, "hdrUiNits").map(|v| v.clamp(80, 500) as u32));
    set!(settings.horizon.hdr_lighting, flag(json, "hdrLighting"));
    set!(settings.horizon.hdr_smooth_gradients, flag(json, "hdrSmoothGradients"));
    // Extras and mods
    set!(settings.extras.fullbright, flag(json, "fullbright"));
    set!(settings.extras.dynamic_lights, flag(json, "dynamicLights"));
    set!(settings.extras.own_name_tag, flag(json, "ownNameTag"));
    set!(settings.extras.hide_block_particles, flag(json, "hideBlockParticles"));
    set!(settings.extras.auto_reconnect, flag(json, "autoReconnect"));
    set!(settings.extras.mouse_tweaks, flag(json, "mouseTweaks"));
    set!(settings.extras.sort, flag(json, "inventorySort"));
    set!(settings.extras.chat_animation, flag(json, "smoothChat"));
    set!(settings.extras.pregen, flag(json, "pregen"));
    set!(settings.extras.fps, flag(json, "mod_fps"));
    set!(settings.extras.keystrokes, flag(json, "mod_keystrokes"));
    set!(settings.extras.zoom, flag(json, "mod_zoom"));
    set!(settings.extras.minimap, flag(json, "mod_minimap"));
    // Touch controls
    let touch = &mut settings.touch;
    set!(touch.keyboard_panning, flag(json, "keyboardPanning"));
    set!(touch.button_scale, int(json, "buttonScale").map(|v| v.clamp(50, 500) as f32 / 100.0));
    set!(touch.gyroscope, flag(json, "gyroEnable"));
    set!(touch.gyro_sensitivity, int(json, "gyroSensitivity").map(|v| v.clamp(50, 300) as f32 / 100.0));
    set!(touch.virtual_mouse_enabled, flag(json, "mouseStart"));
    set!(touch.virtual_mouse_sensitivity, int(json, "mouseSpeed").map(|v| v.clamp(25, 300) as f32 / 100.0));
    set!(touch.virtual_mouse_pointer_scale, int(json, "mouseScale").map(|v| v.clamp(25, 300) as f32 / 100.0));
    set!(touch.vibrate, int(json, "vibrateLength").map(|v| v > 0));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_keys_land_in_the_settings_and_unknown_ones_are_ignored() {
        let mut settings = Settings::default();
        let before = settings.max_fps;
        let json = serde_json::json!({
            "renderDistance": 20,
            "fxaa": true,
            "attackIndicator": "Hotbar",
            "gamma": 25,
            "mod_minimap": true,
            "somethingNew": 7
        });
        apply(&mut settings, &json);
        assert_eq!(settings.render_distance, 20);
        assert_eq!(settings.attack_indicator, 2);
        assert!((settings.gamma - 0.25).abs() < 1e-6);
        assert!(settings.horizon.fxaa && settings.extras.minimap);
        assert_eq!(settings.max_fps, before);
    }
}
