//! LED latch: toggle-on-press and mirror modes. Output is a button-indexed lit mask.
//! Mapping lit mask → OUT report bytes is pending the Phase 1 LED probe.

use crate::input::BUTTON_MASK;

/// Invalid LED mode configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedConfigError {
    /// A mask has bits beyond the 39 physical buttons.
    MaskOutOfRange,
    /// A button is in both the toggle and mirror masks.
    MaskOverlap,
}

/// Which buttons latch on press (toggle) and which light only while active (mirror).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LedModes {
    toggle: u64,
    mirror: u64,
}

impl LedModes {
    /// Build validated modes.
    ///
    /// # Errors
    /// Out-of-range bits or overlapping masks.
    pub fn new(toggle: u64, mirror: u64) -> Result<Self, LedConfigError> {
        if (toggle | mirror) & !BUTTON_MASK != 0 {
            return Err(LedConfigError::MaskOutOfRange);
        }
        if toggle & mirror != 0 {
            return Err(LedConfigError::MaskOverlap);
        }
        Ok(Self { toggle, mirror })
    }

    /// Toggle mask.
    #[must_use]
    pub const fn toggle(self) -> u64 {
        self.toggle
    }

    /// Mirror mask.
    #[must_use]
    pub const fn mirror(self) -> u64 {
        self.mirror
    }
}

/// Latch state across reports.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LedLatch {
    prev_pressed: u64,
    latched: u64,
}

impl LedLatch {
    /// All LEDs off.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            prev_pressed: 0,
            latched: 0,
        }
    }

    /// Feed the current button mask. Returns the lit mask.
    /// Out-of-range input bits are ignored.
    pub fn update(&mut self, pressed: u64, modes: LedModes) -> u64 {
        let pressed = pressed & BUTTON_MASK;
        let rising = pressed & !self.prev_pressed;
        self.latched ^= rising & modes.toggle;
        self.prev_pressed = pressed;

        let lit = self.latched | (pressed & modes.mirror);
        debug_assert_eq!(self.latched & !modes.toggle, 0, "latch outside toggle mask");
        debug_assert_eq!(lit & !BUTTON_MASK, 0, "lit mask out of range");
        lit
    }

    /// Clear all latched LEDs (connect, or "all off" combo).
    pub fn reset(&mut self) {
        *self = Self::new();
        debug_assert_eq!(self.latched, 0);
        debug_assert_eq!(self.prev_pressed, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HATCH: u64 = 1 << 4;
    const FILTER: u64 = 1 << 34;

    fn modes() -> LedModes {
        LedModes::new(HATCH, FILTER).unwrap()
    }

    #[test]
    fn press_toggles_on_then_off() {
        let mut l = LedLatch::new();
        assert_eq!(l.update(HATCH, modes()), HATCH); // press → on
        assert_eq!(l.update(HATCH, modes()), HATCH); // hold → on
        assert_eq!(l.update(0, modes()), HATCH); // release → on
        assert_eq!(l.update(HATCH, modes()), 0); // press → off
        assert_eq!(l.update(0, modes()), 0);
    }

    #[test]
    fn mirror_follows_switch() {
        let mut l = LedLatch::new();
        assert_eq!(l.update(FILTER, modes()), FILTER);
        assert_eq!(l.update(0, modes()), 0);
    }

    #[test]
    fn reset_clears() {
        let mut l = LedLatch::new();
        let _ = l.update(HATCH, modes());
        l.reset();
        assert_eq!(l.update(0, modes()), 0);
    }

    #[test]
    fn rejects_bad_masks() {
        assert_eq!(
            LedModes::new(HATCH, HATCH),
            Err(LedConfigError::MaskOverlap)
        );
        assert_eq!(
            LedModes::new(1 << 39, 0),
            Err(LedConfigError::MaskOutOfRange)
        );
    }
}
