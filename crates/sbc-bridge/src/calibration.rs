//! Axis calibration file (`sbc-calibration.toml`) → [`Calibration`].
//!
//! Written by `sbc-capture calibrate`. Missing axes keep the full-range default.

use std::path::Path;

use sbc_core::calib::{AXIS_NAMES, CalError};
use sbc_core::{AxisCal, Calibration};
use serde::Deserialize;

const MAX_FILE_BYTES: u64 = 16 * 1024;

/// One `[axis]` table.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct AxisToml {
    min: i32,
    center: Option<i32>,
    max: i32,
}

impl From<AxisToml> for AxisCal {
    fn from(a: AxisToml) -> Self {
        Self {
            min: a.min,
            center: a.center,
            max: a.max,
        }
    }
}

/// Whole file; keys match [`AXIS_NAMES`].
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
struct CalToml {
    aim_x: Option<AxisToml>,
    aim_y: Option<AxisToml>,
    rotation: Option<AxisToml>,
    sight_x: Option<AxisToml>,
    sight_y: Option<AxisToml>,
    throttle: Option<AxisToml>,
    clutch: Option<AxisToml>,
    brake: Option<AxisToml>,
}

impl CalToml {
    const fn ordered(&self) -> [Option<AxisToml>; 8] {
        [
            self.aim_x,
            self.aim_y,
            self.rotation,
            self.sight_x,
            self.sight_y,
            self.throttle,
            self.clutch,
            self.brake,
        ]
    }
}

/// Parse and validate calibration text.
pub fn parse(text: &str) -> Result<Calibration, String> {
    let file: CalToml = toml::from_str(text).map_err(|e| format!("calibration: {e}"))?;
    let mut cal = Calibration::full_range();
    for (slot, given) in cal.axes.iter_mut().zip(file.ordered()) {
        if let Some(a) = given {
            *slot = a.into();
        }
    }
    cal.validate().map_err(|(i, e)| {
        let name = AXIS_NAMES.get(i).copied().unwrap_or("?");
        let why = match e {
            CalError::OutOfRange => "value outside -32768..=65535",
            CalError::BadOrder => "needs min < center < max, at least 256 apart",
        };
        format!("calibration [{name}]: {why}")
    })?;
    debug_assert!(cal.validate().is_ok());
    Ok(cal)
}

/// Load `path` if it exists. `Ok(None)` = no file (use full range).
pub fn load(path: &Path) -> Result<Option<Calibration>, String> {
    if !path.is_file() {
        return Ok(None);
    }
    let len = std::fs::metadata(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .len();
    if len > MAX_FILE_BYTES {
        return Err(format!(
            "{}: larger than {MAX_FILE_BYTES} bytes",
            path.display()
        ));
    }
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse(&text)
        .map(Some)
        .map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_calibration_file_parses() {
        let text = include_str!("../../../config/calibration.toml");
        let cal = parse(text).unwrap();
        assert_ne!(cal, Calibration::full_range());
        assert_eq!(
            cal.axes.first().and_then(|a| a.center),
            None,
            "aim stick: no spring"
        );
        assert_eq!(cal.axes.get(2).and_then(|a| a.center), Some(48));
    }

    #[test]
    fn empty_file_is_full_range() {
        assert_eq!(parse("").unwrap(), Calibration::full_range());
    }

    #[test]
    fn partial_file_keeps_other_defaults() {
        let cal = parse("[brake]\nmin = 4288\nmax = 63168\n").unwrap();
        assert_eq!(
            cal.axes.last().copied(),
            Some(AxisCal::linear(4288, 63_168))
        );
        assert_eq!(cal.axes.first(), Calibration::full_range().axes.first());
    }

    #[test]
    fn rejects_bad_files() {
        assert!(parse("[brake]\nmin = 10\nmax = 10\n").is_err());
        assert!(parse("[aim_x]\nmin = 0\ncenter = 5\nmax = 900\n").is_err());
        assert!(parse("[pedal]\nmin = 0\nmax = 9000\n").is_err());
        assert!(parse("[brake]\nmin = 0\nmax = 9000\nextra = 1\n").is_err());
    }
}
