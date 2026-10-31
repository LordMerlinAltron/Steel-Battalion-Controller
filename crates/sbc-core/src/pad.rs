//! [`SbcState`] → [`PadState`] for the virtual joystick (vJoy).
//! Button layout: `docs/control-map.md`.

use crate::input::{BUTTON_MASK, SbcState, gear_index};

/// vJoy axis minimum.
pub const AXIS_MIN: u16 = 0x0001;
/// vJoy axis maximum.
pub const AXIS_MAX: u16 = 0x8000;
/// Axis count: X, Y, Z, Rx, Ry, Rz, Slider0, Slider1.
pub const AXIS_COUNT: usize = 8;
/// Highest vJoy button used (1-based).
pub const PAD_BUTTON_COUNT: u32 = 74;

/// Bit index (0-based) of gear "R"; R, N, 1..5 follow.
pub const GEAR_BASE_BIT: u32 = 39;
/// Bit index of tuner position 0; 16 positions follow.
pub const TUNER_BASE_BIT: u32 = 46;
/// Bit index of the first pulse slot (tuner up).
pub const PULSE_BASE_BIT: u32 = 62;

const TUNER_POSITIONS: u8 = 16;
const PULSE_SLOTS: usize = 12;
const SLOT_TUNER_UP: usize = 0;
const SLOT_TUNER_DOWN: usize = 1;
/// Raw bits of the 5 toggle switches (physical buttons 35–39).
const TOGGLE_BITS: [u64; 5] = [1 << 34, 1 << 35, 1 << 36, 1 << 37, 1 << 38];
const TOGGLE_ON_SLOTS: [usize; 5] = [2, 3, 4, 5, 6];
const TOGGLE_OFF_SLOTS: [usize; 5] = [7, 8, 9, 10, 11];

/// Virtual joystick output.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PadState {
    /// Axes in vJoy range `AXIS_MIN..=AXIS_MAX`, order X, Y, Z, Rx, Ry, Rz, Sl0, Sl1.
    pub axes: [u16; AXIS_COUNT],
    /// Bit `n` = vJoy button `n + 1`.
    pub buttons: u128,
}

impl PadState {
    /// Centered sticks, released pedals, no buttons. Sent on disconnect.
    #[must_use]
    pub const fn neutral() -> Self {
        let c = 0x4000;
        Self {
            axes: [c, c, c, c, c, AXIS_MIN, AXIS_MIN, AXIS_MIN],
            buttons: 0,
        }
    }
}

/// Stateful mapper (edge detection + pulse timing).
#[derive(Clone, Copy, Debug)]
pub struct PadMapper {
    pulse_ms: u64,
    primed: bool,
    prev_buttons: u64,
    prev_tuner: u8,
    pulse_until: [u64; PULSE_SLOTS],
}

impl PadMapper {
    /// `pulse_ms`: how long synthesized pulse buttons stay pressed. Clamped to 10..=1000.
    #[must_use]
    pub fn new(pulse_ms: u64) -> Self {
        Self {
            pulse_ms: pulse_ms.clamp(10, 1000),
            primed: false,
            prev_buttons: 0,
            prev_tuner: 0,
            pulse_until: [0; PULSE_SLOTS],
        }
    }

    /// Forget edges (call on reconnect).
    pub fn reset(&mut self) {
        *self = Self::new(self.pulse_ms);
        debug_assert!(!self.primed);
    }

    /// Map one decoded report. `now_ms` must be monotonic.
    pub fn map(&mut self, s: &SbcState, now_ms: u64) -> PadState {
        let raw = s.buttons & BUTTON_MASK;
        if self.primed {
            self.arm_toggle_pulses(raw, now_ms);
            self.arm_tuner_pulses(s.tuner, now_ms);
        }
        self.primed = true;
        self.prev_buttons = raw;
        self.prev_tuner = s.tuner;

        let buttons =
            u128::from(raw) | gear_bits(s.gear) | tuner_bits(s.tuner) | self.pulse_bits(now_ms);
        let pad = PadState {
            axes: map_axes(s),
            buttons,
        };
        debug_assert!(
            pad.buttons < bit(PAD_BUTTON_COUNT),
            "button beyond pad range"
        );
        debug_assert!(pad.axes.iter().all(|a| (AXIS_MIN..=AXIS_MAX).contains(a)));
        pad
    }

    fn arm_toggle_pulses(&mut self, raw: u64, now_ms: u64) {
        let rising = raw & !self.prev_buttons;
        let falling = !raw & self.prev_buttons;
        let slots = TOGGLE_ON_SLOTS.iter().zip(TOGGLE_OFF_SLOTS.iter());
        for (b, (on, off)) in TOGGLE_BITS.iter().zip(slots) {
            if rising & b != 0 {
                self.arm(*on, now_ms);
            }
            if falling & b != 0 {
                self.arm(*off, now_ms);
            }
        }
    }

    fn arm_tuner_pulses(&mut self, tuner: u8, now_ms: u64) {
        if tuner >= TUNER_POSITIONS || self.prev_tuner >= TUNER_POSITIONS {
            return;
        }
        let delta = tuner.wrapping_sub(self.prev_tuner) & 0x0F;
        match delta {
            1..=7 => self.arm(SLOT_TUNER_UP, now_ms),
            9..=15 => self.arm(SLOT_TUNER_DOWN, now_ms),
            _ => {}
        }
    }

    fn arm(&mut self, slot: usize, now_ms: u64) {
        if let Some(t) = self.pulse_until.get_mut(slot) {
            *t = now_ms.saturating_add(self.pulse_ms);
        }
    }

    fn pulse_bits(&self, now_ms: u64) -> u128 {
        let mut out = 0u128;
        for (slot, until) in (0u32..).zip(self.pulse_until.iter()) {
            if now_ms < *until {
                out |= bit(PULSE_BASE_BIT.saturating_add(slot));
            }
        }
        out
    }
}

const fn bit(n: u32) -> u128 {
    match 1u128.checked_shl(n) {
        Some(b) => b,
        None => 0,
    }
}

/// Unsigned 16-bit → vJoy range.
const fn scale_u(v: u16) -> u16 {
    (v >> 1).saturating_add(1)
}

/// Signed 16-bit → vJoy range (center ≈ 0x4000).
const fn scale_s(v: i16) -> u16 {
    scale_u(v.cast_unsigned() ^ 0x8000)
}

fn map_axes(s: &SbcState) -> [u16; AXIS_COUNT] {
    [
        scale_u(s.aim_x),
        scale_u(s.aim_y),
        scale_s(s.rotation),
        scale_s(s.sight_x),
        scale_s(s.sight_y),
        scale_u(s.throttle),
        scale_u(s.clutch),
        scale_u(s.brake),
    ]
}

/// Gear raw value → one-hot button.
const fn gear_bits(gear: i8) -> u128 {
    match gear_index(gear) {
        Some(i) => bit(GEAR_BASE_BIT.saturating_add(i)),
        None => 0,
    }
}

fn tuner_bits(tuner: u8) -> u128 {
    if tuner < TUNER_POSITIONS {
        bit(TUNER_BASE_BIT.saturating_add(u32::from(tuner)))
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILTER: u64 = TOGGLE_BITS[0];

    fn state() -> SbcState {
        SbcState {
            gear: -1,
            ..SbcState::default()
        }
    }

    #[test]
    fn axis_scaling_covers_range() {
        assert_eq!(scale_u(0), AXIS_MIN);
        assert_eq!(scale_u(u16::MAX), AXIS_MAX);
        assert_eq!(scale_s(i16::MIN), AXIS_MIN);
        assert_eq!(scale_s(i16::MAX), AXIS_MAX);
        assert_eq!(scale_s(0), 0x4001);
        assert_eq!(scale_s(-1), 0x4000); // no double-wide center bin
    }

    #[test]
    fn gear_and_tuner_one_hot() {
        let mut m = PadMapper::new(50);
        let s = SbcState {
            gear: 3,
            tuner: 5,
            ..state()
        };
        let p = m.map(&s, 0);
        assert_eq!(p.buttons, bit(GEAR_BASE_BIT + 4) | bit(TUNER_BASE_BIT + 5));
    }

    #[test]
    fn unknown_gear_maps_to_nothing() {
        assert_eq!(gear_bits(0), 0);
        assert_eq!(gear_bits(6), 0);
    }

    #[test]
    fn no_pulses_on_first_report() {
        let mut m = PadMapper::new(50);
        let p = m.map(
            &SbcState {
                buttons: FILTER,
                ..state()
            },
            0,
        );
        assert_eq!(p.buttons & bit(PULSE_BASE_BIT + 2), 0);
    }

    #[test]
    fn toggle_on_and_off_pulse_then_expire() {
        let mut m = PadMapper::new(50);
        let _ = m.map(&state(), 0);
        let on = bit(PULSE_BASE_BIT + 2);
        let off = bit(PULSE_BASE_BIT + 7);

        let p = m.map(
            &SbcState {
                buttons: FILTER,
                ..state()
            },
            100,
        );
        assert_ne!(p.buttons & on, 0);
        let p = m.map(
            &SbcState {
                buttons: FILTER,
                ..state()
            },
            160,
        );
        assert_eq!(p.buttons & on, 0, "pulse expired");

        let p = m.map(&state(), 200);
        assert_ne!(p.buttons & off, 0);
    }

    #[test]
    fn tuner_steps_with_wrap() {
        let mut m = PadMapper::new(50);
        let up = bit(PULSE_BASE_BIT);
        let down = bit(PULSE_BASE_BIT + 1);
        let _ = m.map(
            &SbcState {
                tuner: 15,
                ..state()
            },
            0,
        );
        let p = m.map(
            &SbcState {
                tuner: 0,
                ..state()
            },
            10,
        );
        assert_ne!(p.buttons & up, 0, "15 → 0 is a step up");
        let p = m.map(
            &SbcState {
                tuner: 15,
                ..state()
            },
            100,
        );
        assert_ne!(p.buttons & down, 0, "0 → 15 is a step down");
    }

    #[test]
    fn neutral_is_in_range() {
        assert!(
            PadState::neutral()
                .axes
                .iter()
                .all(|a| (AXIS_MIN..=AXIS_MAX).contains(a))
        );
    }
}
