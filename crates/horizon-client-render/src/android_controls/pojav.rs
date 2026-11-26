// AmeStar: Amethyst (PojavLauncher) control layouts, imported as GoyControl presets.
//! Reads the JSON that Amethyst's control editor saves (`mControlDataList`, `mJoystickDataList`)
//! and turns it into GoyControl buttons: positions come from its `dynamicX` / `dynamicY`
//! expressions, keys from its GLFW key codes and its negative special codes.
use super::custom::CustomButton;
use super::layout::{ControlStyle, Control, Placement, PresetMeta, Settings};
use serde::Deserialize;
use std::path::Path;

/// Folder in the game folder that Amethyst layouts are dropped into.
pub const IMPORT_DIR: &str = "amethyst-controls";

/// Amethyst lays its controls out in dp on the phone it was made on; this is the phone height
/// and density that dp is read against.
const REFERENCE_DENSITY: f64 = 2.0;
const REFERENCE_HEIGHT_DP: f64 = 360.0;
/// Phone width over height, when the importer is not told better.
pub const DEFAULT_ASPECT: f32 = 2.2;

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawLayout {
    #[serde(rename = "mControlDataList")]
    buttons: Vec<RawButton>,
    #[serde(rename = "mJoystickDataList")]
    joysticks: Vec<RawJoystick>,
    #[serde(rename = "scaledAt")]
    scaled_at: Option<f64>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawButton {
    name: String,
    keycodes: Vec<i32>,
    #[serde(rename = "dynamicX")]
    dynamic_x: String,
    #[serde(rename = "dynamicY")]
    dynamic_y: String,
    width: f64,
    height: f64,
    opacity: Option<f32>,
    #[serde(rename = "bgColor")]
    bg_color: i64,
    #[serde(rename = "strokeColor")]
    stroke_color: i64,
    #[serde(rename = "strokeWidth")]
    stroke_width: f32,
    #[serde(rename = "cornerRadius")]
    corner_radius: f32,
    #[serde(rename = "isToggle")]
    toggle: bool,
    #[serde(rename = "isSwipeable")]
    swipeable: bool,
    #[serde(rename = "passThruEnabled")]
    pass_through: bool,
    #[serde(rename = "displayInGame")]
    in_game: bool,
    #[serde(rename = "displayInMenu")]
    in_menu: bool,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawJoystick {
    #[serde(rename = "dynamicX")]
    dynamic_x: String,
    #[serde(rename = "dynamicY")]
    dynamic_y: String,
    width: f64,
    height: f64,
    opacity: Option<f32>,
    #[serde(rename = "forwardLock")]
    forward_lock: bool,
}

/// What an Amethyst layout becomes.
#[derive(Debug)]
pub struct Imported {
    pub buttons: Vec<CustomButton>,
    /// The movement joystick, when the layout has one.
    pub joystick: Option<Placement>,
    pub forward_lock: Option<bool>,
    /// Buttons left out: hidden ones, and ones whose keys Starling has no name for.
    pub skipped: usize,
}

/// The values an expression can name, in pixels of the reference phone.
struct Vars {
    screen_width: f64,
    screen_height: f64,
    width: f64,
    height: f64,
    margin: f64,
    preferred_scale: f64,
}

struct Expr<'a> {
    text: &'a [u8],
    at: usize,
    vars: &'a Vars,
}

impl Expr<'_> {
    fn skip(&mut self) {
        while self.text.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }
    fn eat(&mut self, c: u8) -> bool {
        self.skip();
        if self.text.get(self.at) == Some(&c) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn sum(&mut self) -> Option<f64> {
        let mut v = self.product()?;
        loop {
            if self.eat(b'+') {
                v += self.product()?;
            } else if self.eat(b'-') {
                v -= self.product()?;
            } else {
                return Some(v);
            }
        }
    }
    fn product(&mut self) -> Option<f64> {
        let mut v = self.factor()?;
        loop {
            if self.eat(b'*') {
                v *= self.factor()?;
            } else if self.eat(b'/') {
                let d = self.factor()?;
                if d == 0.0 {
                    return None;
                }
                v /= d;
            } else {
                return Some(v);
            }
        }
    }
    fn factor(&mut self) -> Option<f64> {
        self.skip();
        if self.eat(b'-') {
            return Some(-self.factor()?);
        }
        if self.eat(b'(') {
            let v = self.sum()?;
            return self.eat(b')').then_some(v);
        }
        if self.text[self.at..].starts_with(b"${") {
            let end = self.text[self.at..].iter().position(|&c| c == b'}')? + self.at;
            let name = std::str::from_utf8(&self.text[self.at + 2..end]).ok()?;
            self.at = end + 1;
            return match name {
                "screen_width" => Some(self.vars.screen_width),
                "screen_height" => Some(self.vars.screen_height),
                "width" => Some(self.vars.width),
                "height" => Some(self.vars.height),
                "margin" => Some(self.vars.margin),
                "preferred_scale" => Some(self.vars.preferred_scale),
                _ => None,
            };
        }
        if self.text[self.at..].starts_with(b"px(") {
            self.at += 2;
            self.eat(b'(');
            let v = self.sum()?;
            return self
                .eat(b')')
                .then_some(v * REFERENCE_DENSITY);
        }
        let start = self.at;
        while self.text.get(self.at).is_some_and(|c| {
            c.is_ascii_digit() || matches!(c, b'.' | b'e' | b'E')
                || (matches!(c, b'+' | b'-')
                    && matches!(self.text.get(self.at.wrapping_sub(1)), Some(b'e' | b'E')))
        }) {
            self.at += 1;
        }
        std::str::from_utf8(&self.text[start..self.at])
            .ok()?
            .parse()
            .ok()
    }
}

/// Evaluates one `dynamicX` / `dynamicY` expression, in pixels.
fn evaluate(expression: &str, vars: &Vars) -> Option<f64> {
    let mut e = Expr {
        text: expression.as_bytes(),
        at: 0,
        vars,
    };
    let v = e.sum()?;
    e.skip();
    (e.at == e.text.len() && v.is_finite()).then_some(v)
}

/// The Starling key name for one Amethyst key code: a GLFW key code, or one of Amethyst's own
/// negative codes (mouse buttons, scroll, the keyboard, the virtual mouse and the GUI).
fn token(code: i32) -> Option<String> {
    let key = |name: &str| Some(format!("key.keyboard.{name}"));
    match code {
        -1 => Some("mouse.keyboard".into()),
        -2 => key("f1"),
        -3 => Some("key.mouse.left".into()),
        -4 => Some("key.mouse.right".into()),
        -5 => Some("mouse.virtual.toggle".into()),
        -6 => Some("key.mouse.middle".into()),
        -7 => Some("mouse.scroll.up".into()),
        -8 => Some("mouse.scroll.down".into()),
        // -9 is Amethyst's own menu button; AmeStar has the handle at the top middle instead.
        48..=57 | 65..=90 => key(&(code as u8 as char).to_ascii_lowercase().to_string()),
        290..=301 => key(&format!("f{}", code - 289)),
        320..=329 => key(&format!("keypad.{}", code - 320)),
        _ => key(match code {
            32 => "space",
            39 => "apostrophe",
            44 => "comma",
            45 => "minus",
            46 => "period",
            47 => "slash",
            59 => "semicolon",
            61 => "equal",
            91 => "left.bracket",
            92 => "backslash",
            93 => "right.bracket",
            96 => "grave.accent",
            256 => "escape",
            257 => "enter",
            258 => "tab",
            259 => "backspace",
            260 => "insert",
            261 => "delete",
            262 => "right",
            263 => "left",
            264 => "down",
            265 => "up",
            266 => "page.up",
            267 => "page.down",
            268 => "home",
            269 => "end",
            280 => "caps.lock",
            281 => "scroll.lock",
            282 => "num.lock",
            283 => "print.screen",
            284 => "pause",
            330 => "keypad.period",
            331 => "keypad.divide",
            332 => "keypad.multiply",
            333 => "keypad.subtract",
            334 => "keypad.add",
            335 => "keypad.enter",
            336 => "keypad.equal",
            340 => "left.shift",
            341 => "left.control",
            342 => "left.alt",
            343 => "left.win",
            344 => "right.shift",
            345 => "right.control",
            346 => "right.alt",
            347 => "right.win",
            348 => "menu",
            _ => return None,
        }),
    }
}

/// A box in the reference phone's pixels: where an expression puts it, and its size.
struct Boxed {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

fn place(
    dynamic_x: &str,
    dynamic_y: &str,
    width_dp: f64,
    height_dp: f64,
    scaled_at: f64,
    aspect: f64,
) -> Option<(Boxed, [f64; 2])> {
    let screen_height = REFERENCE_HEIGHT_DP * REFERENCE_DENSITY;
    let screen_width = screen_height * aspect;
    // Sizes are in dp at the button scale the layout was made at (`scaledAt`).
    let w = width_dp * REFERENCE_DENSITY;
    let h = height_dp * REFERENCE_DENSITY;
    let vars = Vars {
        screen_width,
        screen_height,
        width: w,
        height: h,
        margin: 2.0 * REFERENCE_DENSITY,
        preferred_scale: scaled_at,
    };
    let x = evaluate(dynamic_x, &vars)?;
    let y = evaluate(dynamic_y, &vars)?;
    Some((Boxed { x, y, w, h }, [screen_width, screen_height]))
}

fn argb(v: i64) -> u32 {
    v as i32 as u32
}

/// Reads an Amethyst layout. `aspect` is the screen's width over its height.
pub fn convert(json: &str, aspect: f32) -> Result<Imported, String> {
    let raw: RawLayout = serde_json::from_str(json).map_err(|e| e.to_string())?;
    if raw.buttons.is_empty() && raw.joysticks.is_empty() {
        return Err("not an Amethyst control layout".into());
    }
    let scaled_at = raw.scaled_at.filter(|s| *s > 0.0).unwrap_or(100.0);
    let aspect = f64::from(aspect.clamp(1.0, 4.0));
    let mut buttons = Vec::new();
    let mut skipped = 0;
    for b in &raw.buttons {
        if !b.in_game && !b.in_menu {
            skipped += 1;
            continue;
        }
        let tokens: Vec<String> = b
            .keycodes
            .iter()
            .filter(|c| **c != 0)
            .filter_map(|c| token(*c))
            .filter(|t| CustomButton::valid_token(t))
            .collect();
        let (Some(first), Some((at, screen))) = (
            tokens.first(),
            place(&b.dynamic_x, &b.dynamic_y, b.width, b.height, scaled_at, aspect),
        ) else {
            skipped += 1;
            continue;
        };
        if at.w <= 0.0 || at.h <= 0.0 {
            skipped += 1;
            continue;
        }
        let mut button = CustomButton::new(first.clone(), buttons.len());
        button.second_key = tokens.get(1).cloned().unwrap_or_default();
        button.third_key = tokens.get(2).cloned().unwrap_or_default();
        button.custom_label = b.name.trim().to_owned();
        let mut p = Placement::new(
            ((at.x + at.w / 2.0) / screen[0]) as f32,
            ((at.y + at.h / 2.0) / screen[1]) as f32,
            (at.h / screen[1]) as f32,
        );
        p.width_ratio = (at.w / at.h) as f32;
        p.opacity = b.opacity.unwrap_or(1.0).clamp(0.0, 1.0);
        p.toggleable = b.toggle;
        p.swipeable = b.swipeable;
        p.passthrough = b.pass_through;
        p.use_goy_style = true;
        p.shape = u8::from(b.corner_radius < 1.0);
        p.roundness = b.corner_radius.clamp(0.0, 50.0);
        p.stroke_roundness = p.roundness;
        p.stroke_size = b.stroke_width.clamp(0.0, 8.0);
        p.stroke_color = argb(b.stroke_color);
        p.button_color = argb(b.bg_color);
        p.visible_in_game = b.in_game;
        p.visible_in_menu = b.in_menu;
        button.layouts = [p; 3];
        buttons.push(button);
    }
    let joystick = raw.joysticks.first().and_then(|j| {
        let (at, screen) = place(&j.dynamic_x, &j.dynamic_y, j.width, j.height, scaled_at, aspect)?;
        let mut p = Placement::new(
            ((at.x + at.w / 2.0) / screen[0]) as f32,
            ((at.y + at.h / 2.0) / screen[1]) as f32,
            (at.h / screen[1]) as f32,
        );
        p.opacity = j.opacity.unwrap_or(1.0).clamp(0.0, 1.0);
        Some(p)
    });
    Ok(Imported {
        buttons,
        joystick,
        forward_lock: raw.joysticks.first().map(|j| j.forward_lock),
        skipped,
    })
}

impl Settings {
    /// Turns Amethyst layouts into GoyControl presets saved in `game_home`, and renames each file
    /// `.json.imported` (`.json.failed` when it is not an Amethyst layout), so that dropping a
    /// layout in a folder is all an import takes. Every `.json` in an `amethyst-controls/`
    /// folder counts, and so does a loose `.json` in the folder itself that is an Amethyst layout
    /// (other files there are left alone). The folders searched are `game_home` and each of
    /// `also_in` (the game's main folder, which is above a profile's own folder); the
    /// `amethyst-controls/` folder is made in each, so there is somewhere to drop the file.
    pub fn import_amethyst_controls(game_home: &Path, also_in: &[&Path]) -> Vec<PresetMeta> {
        let mut made = Vec::new();
        for root in std::iter::once(game_home).chain(also_in.iter().copied()) {
            let dir = root.join(IMPORT_DIR);
            let _ = std::fs::create_dir_all(&dir);
            for (folder, only_layouts) in [(&dir, false), (&root.to_path_buf(), true)] {
                made.extend(Self::import_folder(game_home, folder, only_layouts));
            }
        }
        made
    }

    fn import_folder(game_home: &Path, folder: &Path, only_layouts: bool) -> Vec<PresetMeta> {
        let mut made = Vec::new();
        for entry in std::fs::read_dir(folder).into_iter().flatten().flatten() {
            let path = entry.path();
            let Some(stem) = path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_suffix(".json"))
                .map(str::to_owned)
            else {
                continue;
            };
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            if only_layouts && !text.contains("mControlDataList") {
                continue;
            }
            // Renamed first: the preset helpers below list presets, which looks here again.
            let imported = path.with_file_name(format!("{stem}.json.imported"));
            if std::fs::rename(&path, &imported).is_err() {
                continue;
            }
            let Ok(layout) = convert(&text, DEFAULT_ASPECT) else {
                let _ = std::fs::rename(&imported, path.with_file_name(format!("{stem}.json.failed")));
                continue;
            };
            if let Ok(meta) = Self::save_imported(game_home, &stem, layout) {
                made.push(meta);
            }
        }
        made
    }

    /// Puts an Amethyst layout on these settings: its buttons, its movement joystick and its
    /// forward lock replace the ones here.
    pub fn apply_amethyst_layout(&mut self, layout: Imported) {
        self.custom_buttons = layout.buttons;
        if let Some(lock) = layout.forward_lock {
            self.forward_lock = lock;
        }
        if let Some(stick) = layout.joystick {
            let slot = self.slot(Control::Move);
            for layouts in &mut self.layouts {
                let kept = layouts[slot];
                layouts[slot] = Placement {
                    center: stick.center,
                    size: stick.size,
                    opacity: stick.opacity,
                    ..kept
                };
            }
        }
    }

    fn save_imported(
        game_home: &Path,
        stem: &str,
        layout: Imported,
    ) -> std::io::Result<PresetMeta> {
        let style = ControlStyle::GoyControl;
        Self::ensure_system_storage(game_home, style);
        let mut s = Self::default_for_style(style);
        s.apply_amethyst_layout(layout);
        let shown: String = format!("Amethyst: {stem}").chars().take(44).collect();
        let name = Self::unique_preset_name(game_home, style, &shown);
        let id = Self::allocate_unique_preset_id(game_home, style, &name);
        s.preset_id = id.clone();
        s.preset_name = name.clone();
        s.preset_description = "Imported from Amethyst".into();
        s.sanitize();
        s.save_to(&Self::system_dir(game_home, style).join(format!("{id}.json")))?;
        Ok(PresetMeta {
            id,
            name,
            description: s.preset_description,
            system: style,
            is_default: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars() -> Vars {
        Vars {
            screen_width: 1584.0,
            screen_height: 720.0,
            width: 108.0,
            height: 86.0,
            margin: 4.0,
            preferred_scale: 90.0,
        }
    }

    #[test]
    fn expressions_follow_amethysts_variables_and_px() {
        let v = vars();
        assert_eq!(evaluate("1.0 * ${screen_width} - ${width}", &v), Some(1476.0));
        // px(dp) is dp times the density (2), scaled by the button scale it was made at.
        assert_eq!(
            evaluate("0.5 * ${screen_height} - (px(40.0) / 80.0 * ${preferred_scale})", &v),
            Some(360.0 - 90.0)
        );
        assert_eq!(evaluate("9.259259E-4 * ${screen_height}", &v).map(|x| (x * 1000.0).round()), Some(667.0));
        assert_eq!(evaluate("1 / 0", &v), None);
        assert_eq!(evaluate("${nonsense}", &v), None);
    }

    #[test]
    fn amethyst_keys_become_starling_keys() {
        assert_eq!(token(-3).as_deref(), Some("key.mouse.left"));
        assert_eq!(token(340).as_deref(), Some("key.keyboard.left.shift"));
        assert_eq!(token(294).as_deref(), Some("key.keyboard.f5"));
        assert_eq!(token(69).as_deref(), Some("key.keyboard.e"));
        assert_eq!(token(-9), None);
        assert_eq!(token(9999), None);
        for code in [-6, -5, -4, -3, -2, -1, 32, 49, 84, 92, 256, 258, 265, 280, 330, 341, 344] {
            let t = token(code).unwrap();
            assert!(CustomButton::valid_token(&t), "{code} -> {t}");
        }
    }

    #[test]
    fn a_layout_becomes_goycontrol_buttons_and_a_joystick() {
        let json = r#"{
          "mControlDataList": [
            {"name":"shift ","keycodes":[340,0,0,0],"dynamicX":"0.7233251 * ${screen_width} - ${width}",
             "dynamicY":"0.20833333 * ${screen_height}","width":76.5,"height":56.0,"opacity":1.0,
             "bgColor":0,"strokeColor":-1,"strokeWidth":0.9,"cornerRadius":0.0,"isToggle":true,
             "isSwipeable":true,"passThruEnabled":false,"displayInGame":true,"displayInMenu":true},
            {"name":"hidden","keycodes":[56,32,-4,49],"dynamicX":"0","dynamicY":"0","width":60.0,"height":60.0,
             "displayInGame":false,"displayInMenu":false},
            {"name":"odd","keycodes":[9999],"dynamicX":"0","dynamicY":"0","width":60.0,"height":60.0,
             "displayInGame":true,"displayInMenu":true}
          ],
          "mJoystickDataList": [
            {"dynamicX":"0.1 * ${screen_width}","dynamicY":"0.9 * ${screen_height} - ${height}",
             "width":131.5,"height":131.5,"opacity":0.3,"forwardLock":true}
          ],
          "scaledAt": 90.0, "version": 8
        }"#;
        let out = convert(json, 2.2).unwrap();
        assert_eq!(out.buttons.len(), 1);
        assert_eq!(out.skipped, 2);
        let b = &out.buttons[0];
        assert_eq!(b.key, "key.keyboard.left.shift");
        assert_eq!(b.custom_label, "shift");
        let p = b.layouts[0];
        assert!(p.toggleable && p.swipeable && p.use_goy_style);
        // 56 dp * 2 = 112 px high on a 720 px screen.
        assert!((p.size - 112.0 / 720.0).abs() < 1e-4);
        assert!((p.width_ratio - 76.5 / 56.0).abs() < 1e-4);
        // Right edge at 0.7233251 of the width, so the centre sits half a width (153 px) left of it.
        let width = 1584.0;
        assert!((p.center[0] - (0.7233251 * width - 153.0 / 2.0) / width).abs() < 1e-3);
        assert_eq!(out.forward_lock, Some(true));
        let stick = out.joystick.unwrap();
        assert!((stick.size - 263.0 / 720.0).abs() < 1e-4);
        assert!((stick.center[1] - (0.9 * 720.0 - 263.0 / 2.0) / 720.0).abs() < 1e-3);
    }

    #[test]
    fn a_layout_dropped_in_the_folder_becomes_a_preset_once() {
        let home = std::env::temp_dir().join(format!("amestar-import-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let dir = home.join(IMPORT_DIR);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("mine.json"),
            r#"{"mControlDataList":[{"name":"jump","keycodes":[32],"dynamicX":"0.5 * ${screen_width}",
            "dynamicY":"0.5 * ${screen_height}","width":60.0,"height":60.0,"displayInGame":true,
            "displayInMenu":true}],"mJoystickDataList":[],"scaledAt":100.0}"#,
        )
        .unwrap();
        std::fs::write(dir.join("junk.json"), "{}").unwrap();
        let made = Settings::import_amethyst_controls(&home, &[]);
        assert_eq!(made.len(), 1);
        assert_eq!(made[0].name, "Amethyst: mine");
        let loaded = Settings::load_preset(&home, ControlStyle::GoyControl, &made[0].id);
        assert_eq!(loaded.custom_buttons.len(), 1);
        assert_eq!(loaded.custom_buttons[0].key, "key.keyboard.space");
        assert!(dir.join("mine.json.imported").exists());
        assert!(dir.join("junk.json.failed").exists());
        assert!(Settings::import_amethyst_controls(&home, &[]).is_empty());
        let _ = std::fs::remove_dir_all(&home);
    }
    #[test]
    fn a_loose_layout_in_the_main_folder_is_found_and_other_json_is_left_alone() {
        let root = std::env::temp_dir().join(format!("amestar-root-{}", std::process::id()));
        let profile = root.join("profile");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&profile).unwrap();
        std::fs::write(
            root.join("pad.json"),
            r#"{"mControlDataList":[{"name":"a","keycodes":[65],"dynamicX":"0","dynamicY":"0",
            "width":50.0,"height":50.0,"displayInGame":true,"displayInMenu":true}]}"#,
        )
        .unwrap();
        std::fs::write(root.join("servers.json"), "{\"a\":1}").unwrap();
        let made = Settings::import_amethyst_controls(&profile, &[root.as_path()]);
        assert_eq!(made.len(), 1);
        assert!(root.join("servers.json").exists());
        assert!(root.join("pad.json.imported").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn other_json_is_refused() {
        assert!(convert("{}", 2.2).is_err());
        assert!(convert("[1, 2]", 2.2).is_err());
    }
}
