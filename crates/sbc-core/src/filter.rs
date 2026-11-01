//! Input filter: removes switch bounce and tuner glitches before mapping.
//!
//! Hardware evidence (`logs/p1-watch.txt`, 2026-11-01): toggle switches bounce for
//! 10–70 ms per flip; the tuner dial reported one transient wrong code for 60 ms.
//! Buttons, gear and axes pass through unchanged (no bounce seen; no added latency).

use crate::input::{SbcState, TOGGLE_MASK};

/// Number of toggle switches (physical buttons 35–39, raw bits 34–38).
const TOGGLE_COUNT: usize = 5;
/// Raw bit of the first toggle switch.
const TOGGLE_FIRST_BIT: u32 = 34;
/// Upper bound for both filter times (ms).
pub const FILTER_MAX_MS: u64 = 1000;

/// Filter settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FilterTimes {
    /// After a toggle changes, ignore further changes for this long (ms). 0 = off.
    pub toggle_ms: u64,
    /// A new tuner position must hold this long before it is accepted (ms). 0 = off.
    pub tuner_ms: u64,
}

impl Default for FilterTimes {
    fn default() -> Self {
        Self {
            toggle_ms: 80,
            tuner_ms: 100,
        }
    }
}

/// Stateful filter. Feed every report (and repeat the last one on read timeouts).
#[derive(Clone, Copy, Debug)]
pub struct InputFilter {
    times: FilterTimes,
    primed: bool,
    /// Accepted toggle bits (subset of [`TOGGLE_MASK`]).
    toggles: u64,
    /// Per toggle: changes ignored until this time.
    locked_until: [u64; TOGGLE_COUNT],
    /// Accepted tuner position.
    tuner: u8,
    /// Candidate tuner position and when it first appeared.
    tuner_candidate: Option<(u8, u64)>,
}

impl InputFilter {
    /// New filter. Times are clamped to `0..=FILTER_MAX_MS`.
    #[must_use]
    pub fn new(times: FilterTimes) -> Self {
        let times = FilterTimes {
            toggle_ms: times.toggle_ms.min(FILTER_MAX_MS),
            tuner_ms: times.tuner_ms.min(FILTER_MAX_MS),
        };
        let f = Self {
            times,
            primed: false,
            toggles: 0,
            locked_until: [0; TOGGLE_COUNT],
            tuner: 0,
            tuner_candidate: None,
        };
        debug_assert!(f.times.toggle_ms <= FILTER_MAX_MS);
        debug_assert!(f.times.tuner_ms <= FILTER_MAX_MS);
        f
    }

    /// Forget all state (call on reconnect).
    pub fn reset(&mut self) {
        *self = Self::new(self.times);
        debug_assert!(!self.primed);
    }

    /// Filter one report. `now_ms` must be monotonic.
    pub fn apply(&mut self, raw: &SbcState, now_ms: u64) -> SbcState {
        if self.primed {
            self.step_toggles(raw.buttons & TOGGLE_MASK, now_ms);
            self.step_tuner(raw.tuner, now_ms);
        } else {
            self.toggles = raw.buttons & TOGGLE_MASK;
            self.tuner = raw.tuner;
            self.primed = true;
        }
        let out = SbcState {
            buttons: (raw.buttons & !TOGGLE_MASK) | self.toggles,
            tuner: self.tuner,
            ..*raw
        };
        debug_assert_eq!(out.buttons & !TOGGLE_MASK, raw.buttons & !TOGGLE_MASK);
        debug_assert_eq!(self.toggles & !TOGGLE_MASK, 0);
        out
    }

    /// Leading edge + lockout: a flip is accepted at once, bounces inside the lockout are
    /// ignored, and the switch is re-read when the lockout ends.
    fn step_toggles(&mut self, raw_toggles: u64, now_ms: u64) {
        for (i, until) in (TOGGLE_FIRST_BIT..).zip(self.locked_until.iter_mut()) {
            let mask = 1u64 << i;
            let differs = (raw_toggles ^ self.toggles) & mask != 0;
            if differs && now_ms >= *until {
                self.toggles ^= mask;
                *until = now_ms.saturating_add(self.times.toggle_ms);
            }
        }
    }

    /// Settle filter: a new position is accepted only after it holds for `tuner_ms`.
    fn step_tuner(&mut self, raw: u8, now_ms: u64) {
        if raw == self.tuner {
            self.tuner_candidate = None;
            return;
        }
        let since = match self.tuner_candidate {
            Some((v, t)) if v == raw => t,
            _ => {
                self.tuner_candidate = Some((raw, now_ms));
                now_ms
            }
        };
        if now_ms.saturating_sub(since) >= self.times.tuner_ms {
            self.tuner = raw;
            self.tuner_candidate = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILTER_SW: u64 = 1 << 34;
    const BUFFER_SW: u64 = 1 << 37;

    fn with(buttons: u64, tuner: u8) -> SbcState {
        SbcState {
            buttons,
            tuner,
            gear: -1,
            ..SbcState::default()
        }
    }

    #[test]
    fn first_report_passes_through() {
        let mut f = InputFilter::new(FilterTimes::default());
        let s = with(FILTER_SW | 1, 7);
        assert_eq!(f.apply(&s, 0), s);
    }

    #[test]
    fn plain_buttons_are_never_delayed() {
        let mut f = InputFilter::new(FilterTimes::default());
        let _ = f.apply(&with(0, 0), 0);
        assert_eq!(f.apply(&with(1, 0), 1).buttons, 1);
        assert_eq!(f.apply(&with(0, 0), 2).buttons, 0);
    }

    #[test]
    fn toggle_bounce_from_hardware_log_is_one_flip() {
        // p1-watch.txt: ToggleBufferMaterial + 48719, - 48731, + 48791 (one flip ON).
        let mut f = InputFilter::new(FilterTimes::default());
        let _ = f.apply(&with(0, 0), 48_700);
        assert_eq!(f.apply(&with(BUFFER_SW, 0), 48_719).buttons, BUFFER_SW);
        assert_eq!(f.apply(&with(0, 0), 48_731).buttons, BUFFER_SW);
        assert_eq!(f.apply(&with(BUFFER_SW, 0), 48_791).buttons, BUFFER_SW);
        assert_eq!(f.apply(&with(BUFFER_SW, 0), 48_900).buttons, BUFFER_SW);
    }

    #[test]
    fn toggle_resyncs_after_lockout() {
        // Bounce ends OFF inside the lockout: the real state wins once the lockout ends.
        let mut f = InputFilter::new(FilterTimes::default());
        let _ = f.apply(&with(0, 0), 0);
        assert_eq!(f.apply(&with(FILTER_SW, 0), 10).buttons, FILTER_SW);
        assert_eq!(f.apply(&with(0, 0), 20).buttons, FILTER_SW);
        assert_eq!(f.apply(&with(0, 0), 89).buttons, FILTER_SW);
        assert_eq!(f.apply(&with(0, 0), 90).buttons, 0);
    }

    #[test]
    fn tuner_glitch_from_hardware_log_is_ignored() {
        // p1-watch.txt: 13 @92968, 10 @93600, 13 @93660, 9 @93668, 14 @93680, 15 @94508.
        let mut f = InputFilter::new(FilterTimes::default());
        let _ = f.apply(&with(0, 13), 92_968);
        assert_eq!(f.apply(&with(0, 10), 93_600).tuner, 13);
        assert_eq!(f.apply(&with(0, 10), 93_656).tuner, 13);
        assert_eq!(f.apply(&with(0, 13), 93_660).tuner, 13);
        assert_eq!(f.apply(&with(0, 9), 93_668).tuner, 13);
        assert_eq!(f.apply(&with(0, 14), 93_680).tuner, 13);
        assert_eq!(f.apply(&with(0, 14), 93_779).tuner, 13);
        assert_eq!(f.apply(&with(0, 14), 93_780).tuner, 14);
        let _ = f.apply(&with(0, 15), 94_508);
        assert_eq!(f.apply(&with(0, 15), 94_608).tuner, 15);
    }

    #[test]
    fn zero_times_disable_filtering() {
        let mut f = InputFilter::new(FilterTimes {
            toggle_ms: 0,
            tuner_ms: 0,
        });
        let _ = f.apply(&with(0, 0), 0);
        assert_eq!(f.apply(&with(FILTER_SW, 3), 1), with(FILTER_SW, 3));
        assert_eq!(f.apply(&with(0, 4), 2), with(0, 4));
    }

    #[test]
    fn reset_forgets_state() {
        let mut f = InputFilter::new(FilterTimes::default());
        let _ = f.apply(&with(FILTER_SW, 5), 0);
        f.reset();
        assert_eq!(f.apply(&with(0, 9), 1), with(0, 9));
    }
}
