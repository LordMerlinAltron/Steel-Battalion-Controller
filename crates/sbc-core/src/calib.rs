//! Axis calibration: raw axis value → vJoy range, per axis.
//!
//! Sticks and levers use three points (min, center, max) so the rest position maps to the
//! vJoy center even when it is not in the middle of the raw range (aim stick: rest ≈ 40 %).
//! Pedals use two points (min = released, max = floored).

use crate::input::SbcState;
use crate::pad::{AXIS_COUNT, AXIS_MAX, AXIS_MIN};

/// vJoy center value.
pub const AXIS_MID: u16 = 0x4000;
/// Smallest allowed raw distance between calibration points.
pub const MIN_SPAN: i32 = 256;
/// Lowest raw value any axis can report (signed axes).
pub const RAW_LOW: i32 = -32_768;
/// Highest raw value any axis can report (unsigned axes).
pub const RAW_HIGH: i32 = 65_535;

/// Axis names in vJoy order X, Y, Z, Rx, Ry, Rz, Slider0, Slider1 (also the TOML keys).
pub const AXIS_NAMES: [&str; AXIS_COUNT] = [
    "aim_x", "aim_y", "rotation", "sight_x", "sight_y", "throttle", "clutch", "brake",
];

/// Why a calibration is rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalError {
    /// A point is outside `RAW_LOW..=RAW_HIGH`.
    OutOfRange,
    /// Points are not ordered `min < center < max` with at least [`MIN_SPAN`] between them.
    BadOrder,
}

/// One axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AxisCal {
    /// Raw value for vJoy minimum.
    pub min: i32,
    /// Raw rest position for vJoy center; `None` = linear min → max (pedals).
    pub center: Option<i32>,
    /// Raw value for vJoy maximum.
    pub max: i32,
}

impl AxisCal {
    /// Two-point (pedal) calibration.
    #[must_use]
    pub const fn linear(min: i32, max: i32) -> Self {
        Self {
            min,
            center: None,
            max,
        }
    }

    /// Three-point (stick) calibration.
    #[must_use]
    pub const fn centered(min: i32, center: i32, max: i32) -> Self {
        Self {
            min,
            center: Some(center),
            max,
        }
    }

    /// Check point order and range.
    ///
    /// # Errors
    /// [`CalError`] describing the first problem.
    pub fn validate(&self) -> Result<(), CalError> {
        let points = [self.min, self.center.unwrap_or(self.min), self.max];
        if points.iter().any(|p| !(RAW_LOW..=RAW_HIGH).contains(p)) {
            return Err(CalError::OutOfRange);
        }
        let ordered = match self.center {
            None => gap(self.min, self.max),
            Some(c) => gap(self.min, c) && gap(c, self.max),
        };
        debug_assert!(self.min >= RAW_LOW && self.max <= RAW_HIGH);
        if ordered {
            Ok(())
        } else {
            Err(CalError::BadOrder)
        }
    }

    /// Map a raw value. Values beyond min/max clamp. Assumes [`Self::validate`] passed.
    #[must_use]
    pub fn map(&self, raw: i32) -> u16 {
        let v = i64::from(raw.clamp(self.min, self.max));
        let min = i64::from(self.min);
        let max = i64::from(self.max);
        let out = match self.center.map(i64::from) {
            None => lerp(
                v.saturating_sub(min),
                max.saturating_sub(min),
                AXIS_MIN,
                AXIS_MAX,
            ),
            Some(c) if v <= c => lerp(
                v.saturating_sub(min),
                c.saturating_sub(min),
                AXIS_MIN,
                AXIS_MID,
            ),
            Some(c) => lerp(
                v.saturating_sub(c),
                max.saturating_sub(c),
                AXIS_MID,
                AXIS_MAX,
            ),
        };
        debug_assert!((AXIS_MIN..=AXIS_MAX).contains(&out));
        out
    }
}

/// True if `hi` is at least [`MIN_SPAN`] above `lo`.
const fn gap(lo: i32, hi: i32) -> bool {
    hi.saturating_sub(lo) >= MIN_SPAN
}

/// `pos` of `span` → `lo..=hi`, clamped.
fn lerp(pos: i64, span: i64, lo: u16, hi: u16) -> u16 {
    let out_span = i64::from(hi).saturating_sub(i64::from(lo));
    let scaled = pos.saturating_mul(out_span).checked_div(span).unwrap_or(0);
    let v = i64::from(lo)
        .saturating_add(scaled)
        .clamp(i64::from(lo), i64::from(hi));
    u16::try_from(v).unwrap_or(lo)
}

/// All axes, vJoy order (see [`AXIS_NAMES`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Calibration {
    /// Per-axis calibration.
    pub axes: [AxisCal; AXIS_COUNT],
}

impl Default for Calibration {
    fn default() -> Self {
        Self::full_range()
    }
}

impl Calibration {
    /// No calibration: full raw range. Signed axes centered on 0; aim stick and pedals linear.
    #[must_use]
    pub const fn full_range() -> Self {
        let u = AxisCal::linear(0, 0xFFFF);
        let s = AxisCal::centered(-32_768, 0, 32_767);
        Self {
            axes: [u, u, s, s, s, u, u, u],
        }
    }

    /// Check every axis.
    ///
    /// # Errors
    /// `(axis index, error)` for the first bad axis.
    pub fn validate(&self) -> Result<(), (usize, CalError)> {
        for (i, a) in self.axes.iter().enumerate() {
            a.validate().map_err(|e| (i, e))?;
        }
        debug_assert_eq!(self.axes.len(), AXIS_NAMES.len());
        Ok(())
    }

    /// Map all axes of one decoded report.
    #[must_use]
    pub fn map(&self, s: &SbcState) -> [u16; AXIS_COUNT] {
        let raw = raw_axes(s);
        let mut out = [AXIS_MIN; AXIS_COUNT];
        for ((o, cal), r) in out.iter_mut().zip(self.axes.iter()).zip(raw) {
            *o = cal.map(r);
        }
        debug_assert!(out.iter().all(|a| (AXIS_MIN..=AXIS_MAX).contains(a)));
        out
    }
}

/// Raw axis values in vJoy order.
#[must_use]
pub fn raw_axes(s: &SbcState) -> [i32; AXIS_COUNT] {
    [
        s.aim_x.into(),
        s.aim_y.into(),
        s.rotation.into(),
        s.sight_x.into(),
        s.sight_y.into(),
        s.throttle.into(),
        s.clutch.into(),
        s.brake.into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_hits_ends_and_clamps() {
        let a = AxisCal::linear(4288, 63_168); // brake, from hardware
        assert_eq!(a.map(4288), AXIS_MIN);
        assert_eq!(a.map(0), AXIS_MIN);
        assert_eq!(a.map(63_168), AXIS_MAX);
        assert_eq!(a.map(65_535), AXIS_MAX);
    }

    #[test]
    fn centered_rest_maps_to_mid() {
        let a = AxisCal::centered(1600, 25_856, 63_168); // aim X, from hardware
        assert_eq!(a.map(25_856), AXIS_MID);
        assert_eq!(a.map(1600), AXIS_MIN);
        assert_eq!(a.map(63_168), AXIS_MAX);
        assert!(a.map(13_728) < AXIS_MID && a.map(13_728) > AXIS_MIN);
    }

    #[test]
    fn validate_rejects_bad_points() {
        assert_eq!(AxisCal::linear(10, 10).validate(), Err(CalError::BadOrder));
        assert_eq!(
            AxisCal::centered(0, 100, 5000).validate(),
            Err(CalError::BadOrder)
        );
        assert_eq!(
            AxisCal::linear(0, 70_000).validate(),
            Err(CalError::OutOfRange)
        );
        assert_eq!(
            AxisCal::centered(-40_000, 0, 10).validate(),
            Err(CalError::OutOfRange)
        );
        assert!(Calibration::full_range().validate().is_ok());
    }

    #[test]
    fn full_range_matches_raw_extremes() {
        let c = Calibration::full_range();
        let lo = SbcState {
            rotation: i16::MIN,
            ..SbcState::default()
        };
        let hi = SbcState {
            aim_x: u16::MAX,
            rotation: i16::MAX,
            brake: u16::MAX,
            ..SbcState::default()
        };
        assert_eq!(c.map(&lo)[2], AXIS_MIN);
        assert_eq!(c.map(&lo)[3], AXIS_MID, "sight at 0 is centered");
        assert_eq!(c.map(&hi)[0], AXIS_MAX);
        assert_eq!(c.map(&hi)[2], AXIS_MAX);
        assert_eq!(c.map(&hi)[7], AXIS_MAX);
    }
}
