//! Bridge configuration: TOML file → validated [`Config`].

use std::path::{Path, PathBuf};

use sbc_core::filter::FILTER_MAX_MS;
use sbc_core::ledout::{LED_BUTTON_MASK, LED_MAX_LEVEL};
use sbc_core::{Calibration, FilterTimes, GearLedStyle, LedLevels, LedModes};
use serde::Deserialize;

const MAX_CONFIG_BYTES: u64 = 64 * 1024;

/// What panel-button LEDs do.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PanelLedMode {
    /// Press → on, press again → off.
    #[default]
    Toggle,
    /// Lit only while held.
    Momentary,
    /// Never lit.
    Off,
}

/// Gear LED style as written in TOML.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GearMode {
    /// Current gear only.
    #[default]
    Position,
    /// N through current gear.
    Progressive,
    /// Off.
    Off,
}

/// `[vjoy]`
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct VjoySection {
    /// vJoy device id (1..=16).
    pub device_id: u8,
    /// Synthesized pulse length (ms).
    pub pulse_ms: u64,
    /// `vJoyInterface.dll` path; empty = default install path.
    pub dll_path: String,
}

impl Default for VjoySection {
    fn default() -> Self {
        Self {
            device_id: 1,
            pulse_ms: 60,
            dll_path: String::new(),
        }
    }
}

/// `[usb]`
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct UsbSection {
    /// Read timeout per poll (ms).
    pub read_timeout_ms: u64,
    /// Wait between reconnect attempts (ms).
    pub reconnect_ms: u64,
}

impl Default for UsbSection {
    fn default() -> Self {
        Self {
            read_timeout_ms: 100,
            reconnect_ms: 1000,
        }
    }
}

/// `[input]`
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct InputSection {
    /// Toggle-switch debounce (ms); 0 = off.
    pub toggle_debounce_ms: u64,
    /// Tuner dial settle time (ms); 0 = off.
    pub tuner_settle_ms: u64,
}

impl Default for InputSection {
    fn default() -> Self {
        let t = FilterTimes::default();
        Self {
            toggle_debounce_ms: t.toggle_ms,
            tuner_settle_ms: t.tuner_ms,
        }
    }
}

/// `[leds]`
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct LedSection {
    /// Master switch.
    pub enabled: bool,
    /// Lit brightness 0..=15.
    pub on_level: u8,
    /// Unlit brightness 0..=15.
    pub off_level: u8,
    /// Clear latched LEDs when the controller connects.
    pub reset_on_connect: bool,
    /// Panel button behavior.
    pub panel_buttons: PanelLedMode,
    /// Gear LED style.
    pub gear: GearMode,
    /// Hold Start this long to clear all LEDs (0 = off).
    pub clear_combo_ms: u64,
}

impl Default for LedSection {
    fn default() -> Self {
        Self {
            enabled: true,
            on_level: LED_MAX_LEVEL,
            off_level: 0,
            reset_on_connect: true,
            panel_buttons: PanelLedMode::Toggle,
            gear: GearMode::Position,
            clear_combo_ms: 2000,
        }
    }
}

/// Whole file.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// `[vjoy]`
    pub vjoy: VjoySection,
    /// `[usb]`
    pub usb: UsbSection,
    /// `[input]`
    pub input: InputSection,
    /// `[leds]`
    pub leds: LedSection,
    /// Axis calibration; comes from `sbc-calibration.toml`, not from this file.
    #[serde(skip)]
    pub calibration: Calibration,
}

impl Config {
    /// Parse and validate TOML text.
    pub fn parse(text: &str) -> Result<Self, String> {
        let cfg: Self = toml::from_str(text).map_err(|e| format!("config: {e}"))?;
        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<(), String> {
        let checks: [(bool, &str); 7] = [
            (
                (1..=16).contains(&self.vjoy.device_id),
                "vjoy.device_id must be 1..=16",
            ),
            (
                (10..=1000).contains(&self.vjoy.pulse_ms),
                "vjoy.pulse_ms must be 10..=1000",
            ),
            (
                (10..=1000).contains(&self.usb.read_timeout_ms),
                "usb.read_timeout_ms must be 10..=1000",
            ),
            (
                (100..=10_000).contains(&self.usb.reconnect_ms),
                "usb.reconnect_ms must be 100..=10000",
            ),
            (
                self.leds.on_level <= LED_MAX_LEVEL && self.leds.off_level <= LED_MAX_LEVEL,
                "leds levels must be 0..=15",
            ),
            (
                self.leds.clear_combo_ms <= 10_000,
                "leds.clear_combo_ms must be 0..=10000",
            ),
            (
                self.input.toggle_debounce_ms <= FILTER_MAX_MS
                    && self.input.tuner_settle_ms <= FILTER_MAX_MS,
                "input times must be 0..=1000",
            ),
        ];
        let failed: Vec<&str> = checks
            .iter()
            .filter(|(ok, _)| !ok)
            .map(|(_, m)| *m)
            .collect();
        debug_assert!(failed.len() <= checks.len());
        if failed.is_empty() {
            Ok(())
        } else {
            Err(format!("config: {}", failed.join("; ")))
        }
    }

    /// DLL path to load.
    #[must_use]
    pub fn dll_path(&self) -> PathBuf {
        if self.vjoy.dll_path.trim().is_empty() {
            sbc_vjoy::default_dll_path()
        } else {
            PathBuf::from(self.vjoy.dll_path.trim())
        }
    }

    /// LED latch modes.
    #[must_use]
    pub fn led_modes(&self) -> LedModes {
        let (toggle, mirror) = match self.leds.panel_buttons {
            PanelLedMode::Toggle => (LED_BUTTON_MASK, 0),
            PanelLedMode::Momentary => (0, LED_BUTTON_MASK),
            PanelLedMode::Off => (0, 0),
        };
        LedModes::new(toggle, mirror).unwrap_or_default()
    }

    /// Input filter times.
    #[must_use]
    pub const fn filter_times(&self) -> FilterTimes {
        FilterTimes {
            toggle_ms: self.input.toggle_debounce_ms,
            tuner_ms: self.input.tuner_settle_ms,
        }
    }

    /// LED brightness levels.
    #[must_use]
    pub const fn led_levels(&self) -> LedLevels {
        LedLevels {
            on: self.leds.on_level,
            off: self.leds.off_level,
        }
    }

    /// Gear LED style.
    #[must_use]
    pub const fn gear_style(&self) -> GearLedStyle {
        match self.leds.gear {
            GearMode::Position => GearLedStyle::Position,
            GearMode::Progressive => GearLedStyle::Progressive,
            GearMode::Off => GearLedStyle::Off,
        }
    }
}

/// Load from the first existing candidate. Returns config and where it came from.
pub fn load(candidates: &[PathBuf]) -> Result<(Config, String), String> {
    for path in candidates.iter().take(8) {
        if path.is_file() {
            return read_file(path).map(|c| (c, path.display().to_string()));
        }
    }
    Ok((Config::default(), "built-in defaults".to_owned()))
}

fn read_file(path: &Path) -> Result<Config, String> {
    let len = std::fs::metadata(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .len();
    if len > MAX_CONFIG_BYTES {
        return Err(format!(
            "{}: larger than {MAX_CONFIG_BYTES} bytes",
            path.display()
        ));
    }
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Config::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_default_file_parses() {
        let text = include_str!("../../../config/default.toml");
        let cfg = Config::parse(text).unwrap();
        assert_eq!(cfg.vjoy.device_id, 1);
        assert_eq!(cfg.leds.panel_buttons, PanelLedMode::Toggle);
        assert_eq!(cfg.filter_times(), FilterTimes::default());
    }

    #[test]
    fn empty_file_is_defaults() {
        assert_eq!(Config::parse("").unwrap(), Config::default());
    }

    #[test]
    fn rejects_bad_values_and_unknown_keys() {
        assert!(Config::parse("[vjoy]\ndevice_id = 0").is_err());
        assert!(Config::parse("[leds]\non_level = 16").is_err());
        assert!(Config::parse("[leds]\npanel_buttons = \"blink\"").is_err());
        assert!(Config::parse("[usb]\ntypo = 1").is_err());
        assert!(Config::parse("[input]\ntoggle_debounce_ms = 5000").is_err());
    }

    #[test]
    fn modes_follow_setting() {
        let mut cfg = Config::default();
        assert_eq!(cfg.led_modes().toggle(), LED_BUTTON_MASK);
        cfg.leds.panel_buttons = PanelLedMode::Momentary;
        assert_eq!(cfg.led_modes().mirror(), LED_BUTTON_MASK);
        cfg.leds.panel_buttons = PanelLedMode::Off;
        assert_eq!(cfg.led_modes(), LedModes::default());
    }
}
