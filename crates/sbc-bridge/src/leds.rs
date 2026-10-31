//! LED driver: latch + clear combo + change detection. Pure (no I/O); the session writes the reports.

use sbc_core::input::gear_index;
use sbc_core::{
    GearLedStyle, LED_REPORT_LEN, LedLatch, LedLevels, LedModes, SbcState, encode_leds,
};

/// Raw bit of the Start button (vJoy 7).
const START_BIT: u64 = 1 << 6;

/// Produces a new OUT report only when the LED picture changes.
#[derive(Clone, Copy, Debug)]
pub struct LedDriver {
    latch: LedLatch,
    modes: LedModes,
    levels: LedLevels,
    gear: GearLedStyle,
    combo_ms: u64,
    start_since: Option<u64>,
    combo_fired: bool,
    /// Last real gear; raw 0 means "between gears" and must not blank the gear LEDs.
    gear_held: i8,
    last: Option<[u8; LED_REPORT_LEN]>,
}

impl LedDriver {
    /// New driver; nothing sent yet.
    #[must_use]
    pub const fn new(
        modes: LedModes,
        levels: LedLevels,
        gear: GearLedStyle,
        combo_ms: u64,
    ) -> Self {
        Self {
            latch: LedLatch::new(),
            modes,
            levels,
            gear,
            combo_ms,
            start_since: None,
            combo_fired: false,
            gear_held: 0,
            last: None,
        }
    }

    /// Report to send at connect (all off with current levels), and remember it.
    pub fn connect_report(&mut self, reset: bool) -> [u8; LED_REPORT_LEN] {
        if reset {
            self.latch.reset();
        }
        let report = encode_leds(0, i8::MIN, GearLedStyle::Off, self.levels);
        self.last = Some(report);
        debug_assert!(self.last.is_some());
        debug_assert!(!reset || self.latch == LedLatch::new());
        report
    }

    /// Feed one input state. `Some(report)` when the LEDs must change.
    pub fn next(&mut self, s: &SbcState, now_ms: u64) -> Option<[u8; LED_REPORT_LEN]> {
        let lit = self.latch.update(s.buttons, self.modes);
        let lit = if self.combo(s.buttons, now_ms) {
            self.latch.clear_holding(s.buttons);
            0
        } else {
            lit
        };
        if gear_index(s.gear).is_some() {
            self.gear_held = s.gear;
        }
        let report = encode_leds(lit, self.gear_held, self.gear, self.levels);
        debug_assert_eq!(report.len(), LED_REPORT_LEN);
        if self.last == Some(report) {
            return None;
        }
        self.last = Some(report);
        Some(report)
    }

    /// True exactly once per Start hold longer than `combo_ms`.
    fn combo(&mut self, buttons: u64, now_ms: u64) -> bool {
        if self.combo_ms == 0 || buttons & START_BIT == 0 {
            self.start_since = None;
            self.combo_fired = false;
            return false;
        }
        let since = *self.start_since.get_or_insert(now_ms);
        if self.combo_fired || now_ms.saturating_sub(since) < self.combo_ms {
            return false;
        }
        self.combo_fired = true;
        debug_assert!(self.start_since.is_some());
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbc_core::ledout::LED_BUTTON_MASK;

    const COMM1: u64 = 1 << 28; // vJoy 29 → LED id 29 → byte 14 high nibble

    fn driver() -> LedDriver {
        let modes = LedModes::new(LED_BUTTON_MASK, 0).unwrap();
        LedDriver::new(modes, LedLevels::default(), GearLedStyle::Off, 2000)
    }

    fn st(buttons: u64) -> SbcState {
        SbcState {
            buttons,
            gear: -1,
            ..SbcState::default()
        }
    }

    #[test]
    fn toggle_sends_only_on_change() {
        let mut d = driver();
        let _ = d.connect_report(true);
        let on = d.next(&st(COMM1), 0).unwrap();
        assert_eq!(on[14], 0xF0);
        assert_eq!(d.next(&st(COMM1), 10), None, "held: no resend");
        assert_eq!(d.next(&st(0), 20), None, "released: still on, no resend");
        let off = d.next(&st(COMM1), 30).unwrap();
        assert_eq!(off[14], 0x00);
    }

    #[test]
    fn start_hold_clears_all() {
        let mut d = driver();
        let _ = d.connect_report(true);
        let _ = d.next(&st(COMM1), 0);
        let _ = d.next(&st(0), 10);
        let _ = d.next(&st(START_BIT), 100); // Start LED toggles on
        let cleared = d.next(&st(START_BIT), 2200).unwrap();
        assert_eq!(cleared, sbc_core::blank_report());
        assert_eq!(d.next(&st(START_BIT), 2300), None, "fires once per hold");
        assert_eq!(d.next(&st(0), 2400), None, "release does not relight");
    }

    #[test]
    fn gear_leds_hold_through_neutral_gap() {
        let modes = LedModes::new(LED_BUTTON_MASK, 0).unwrap();
        let mut d = LedDriver::new(modes, LedLevels::default(), GearLedStyle::Position, 0);
        let _ = d.connect_report(true);
        let third = d
            .next(
                &SbcState {
                    gear: 3,
                    ..SbcState::default()
                },
                0,
            )
            .unwrap();
        assert_eq!(third[19], 0xF0);
        let between = SbcState {
            gear: 0,
            ..SbcState::default()
        };
        assert_eq!(
            d.next(&between, 10),
            None,
            "raw 0 between gears: no LED change"
        );
        let fourth = d
            .next(
                &SbcState {
                    gear: 4,
                    ..SbcState::default()
                },
                20,
            )
            .unwrap();
        assert_eq!((fourth[19], fourth[20]), (0x00, 0x0F));
    }

    #[test]
    fn connect_report_is_all_off() {
        let mut d = driver();
        assert_eq!(d.connect_report(true), sbc_core::blank_report());
        assert_eq!(d.next(&st(0), 0), None);
    }
}
