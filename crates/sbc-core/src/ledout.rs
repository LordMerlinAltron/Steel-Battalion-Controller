//! LED OUT report encoding.
//!
//! Report: 22 bytes on the interrupt OUT endpoint. Byte 0 = `0x00`, byte 1 = length `0x16`.
//! LED id `n` (4..=41) lives in byte `n / 2`: even id → low nibble, odd id → high nibble.
//! Brightness 0..=15. Button LED ids equal vJoy button numbers 4..=33 (raw bit + 1).
//! Gear LEDs: R=35, N=36, 1st..5th=37..41. Byte 21 unused.
//! Sources: xboxdevwiki "Xbox Input Devices", SteelBattalion.NET (facts only).
//! Verify with `sbc-capture led-probe`.

use crate::input::gear_index;

/// OUT report length (XID `bMaxOutputReportSize`).
pub const LED_REPORT_LEN: usize = 22;
/// Fixed report header: report id 0, length.
pub const LED_REPORT_HEADER: [u8; 2] = [0x00, 0x16];
/// Lowest LED id (byte 2, low nibble).
pub const FIRST_LED_ID: u8 = 4;
/// Highest LED id (Gear 5, byte 20, high nibble).
pub const LAST_LED_ID: u8 = 41;
/// Maximum brightness.
pub const LED_MAX_LEVEL: u8 = 15;
/// Raw button bits that have an LED (vJoy 4..=33 → bits 3..=32).
pub const LED_BUTTON_MASK: u64 = ((1u64 << 33) - 1) & !0b111;
/// LED id of gear "R"; N, 1st..5th follow.
pub const GEAR_LED_BASE: u8 = 35;

const BUTTON_BITS: u8 = 39;
const GEAR_POSITIONS: u32 = 7;

/// How gear LEDs show the current gear.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GearLedStyle {
    /// Only the current position.
    #[default]
    Position,
    /// N through the current gear (R lights alone).
    Progressive,
    /// Gear LEDs off.
    Off,
}

/// LED output settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LedLevels {
    /// Brightness for lit LEDs (clamped to 15).
    pub on: u8,
    /// Brightness for unlit LEDs (clamped to 15; non-zero = dim backlight).
    pub off: u8,
}

impl Default for LedLevels {
    fn default() -> Self {
        Self {
            on: LED_MAX_LEVEL,
            off: 0,
        }
    }
}

/// Report with header set and every LED off.
#[must_use]
pub const fn blank_report() -> [u8; LED_REPORT_LEN] {
    let mut r = [0u8; LED_REPORT_LEN];
    r[0] = LED_REPORT_HEADER[0];
    r[1] = LED_REPORT_HEADER[1];
    r
}

/// Set one LED nibble. Ids outside `FIRST_LED_ID..=LAST_LED_ID` are ignored (protects the header).
pub fn set_level(report: &mut [u8; LED_REPORT_LEN], id: u8, level: u8) {
    if !(FIRST_LED_ID..=LAST_LED_ID).contains(&id) {
        return;
    }
    let level = level.min(LED_MAX_LEVEL);
    let Some(byte) = report.get_mut(usize::from(id >> 1)) else {
        return;
    };
    *byte = if id & 1 == 0 {
        (*byte & 0xF0) | level
    } else {
        (*byte & 0x0F) | (level << 4)
    };
    debug_assert!(level <= LED_MAX_LEVEL);
    debug_assert!(usize::from(id >> 1) < LED_REPORT_LEN);
}

/// Build a full report from a lit-button mask (from [`crate::LedLatch`]) and the gear value.
#[must_use]
pub fn encode(lit: u64, gear: i8, style: GearLedStyle, levels: LedLevels) -> [u8; LED_REPORT_LEN] {
    let mut report = blank_report();
    let lit = lit & LED_BUTTON_MASK;
    for b in 0..BUTTON_BITS {
        if LED_BUTTON_MASK & (1u64 << b) != 0 {
            let level = if lit & (1u64 << b) != 0 {
                levels.on
            } else {
                levels.off
            };
            set_level(&mut report, b.saturating_add(1), level);
        }
    }
    let lit_gears = gear_mask(gear, style);
    for g in 0..GEAR_POSITIONS {
        let level = if lit_gears & (1u32 << g) != 0 {
            levels.on
        } else {
            levels.off
        };
        let id = u8::try_from(g).map_or(0, |g| GEAR_LED_BASE.saturating_add(g));
        set_level(&mut report, id, level);
    }
    debug_assert!(lit_gears < (1u32 << GEAR_POSITIONS));
    debug_assert_eq!(report.len(), LED_REPORT_LEN);
    report
}

/// Bit `i` set = gear LED `i` (R=0, N=1, 1st..5th=2..6) lit.
fn gear_mask(gear: i8, style: GearLedStyle) -> u32 {
    let Some(i) = gear_index(gear) else { return 0 };
    match style {
        GearLedStyle::Off => 0,
        GearLedStyle::Position => 1u32 << i,
        GearLedStyle::Progressive if i == 0 => 1,
        // N..=current: bits 1..=i
        GearLedStyle::Progressive => ((1u32 << i) << 1).wrapping_sub(2),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HATCH_BIT: u64 = 1 << 4; // vJoy 5 → LED id 5 → byte 2 high nibble
    const EJECT_BIT: u64 = 1 << 3; // vJoy 4 → LED id 4 → byte 2 low nibble

    fn levels() -> LedLevels {
        LedLevels::default()
    }

    #[test]
    fn mask_covers_vjoy_4_to_33() {
        assert_eq!(LED_BUTTON_MASK.count_ones(), 30);
        assert_eq!(LED_BUTTON_MASK.trailing_zeros(), 3);
        assert_eq!(64 - LED_BUTTON_MASK.leading_zeros(), 33);
    }

    #[test]
    fn nibble_placement() {
        let r = encode(HATCH_BIT, 0, GearLedStyle::Off, levels());
        assert_eq!(r[2], 0xF0);
        let r = encode(EJECT_BIT | HATCH_BIT, 0, GearLedStyle::Off, levels());
        assert_eq!(r[2], 0xFF);
        assert_eq!(r.get(2..).unwrap().iter().filter(|b| **b != 0).count(), 1);
    }

    #[test]
    fn header_and_length_match_xid() {
        let r = encode(
            LED_BUTTON_MASK,
            5,
            GearLedStyle::Progressive,
            LedLevels { on: 15, off: 15 },
        );
        assert_eq!(r.len(), 22);
        assert_eq!((r[0], r[1]), (0x00, 0x16), "header survives every LED on");
        assert_eq!(r[21], 0x00, "byte 21 unused");
    }

    #[test]
    fn wiki_layout_spot_checks() {
        // xboxdevwiki: byte 16 = Comm4 (low) / Comm5 (high); byte 20 = Gear4 / Gear5.
        let comm5 = 1u64 << 32; // vJoy 33
        assert_eq!(encode(comm5, 0, GearLedStyle::Off, levels())[16], 0xF0);
        assert_eq!(encode(0, 5, GearLedStyle::Position, levels())[20], 0xF0);
        assert_eq!(encode(0, 4, GearLedStyle::Position, levels())[20], 0x0F);
    }

    #[test]
    fn set_level_ignores_header_and_unused_ids() {
        let mut r = blank_report();
        for id in [0, 1, 2, 3, 42, 43, 200] {
            set_level(&mut r, id, 15);
        }
        assert_eq!(r, blank_report());
    }

    #[test]
    fn non_led_buttons_ignored() {
        let aim_and_toggles = 0b111 | (0x3F << 33);
        assert_eq!(
            encode(aim_and_toggles, 0, GearLedStyle::Off, levels()),
            blank_report()
        );
    }

    #[test]
    fn gear_position_and_progressive() {
        // 3rd gear (raw 3) → LED id 39 → byte 19 high nibble
        let r = encode(0, 3, GearLedStyle::Position, levels());
        assert_eq!(r[19], 0xF0);
        assert_eq!(r[18], 0x00);
        // progressive 3rd: N(36) 1(37) 2(38) 3(39) → bytes 18, 19 = 0xFF
        let r = encode(0, 3, GearLedStyle::Progressive, levels());
        assert_eq!((r[17], r[18], r[19]), (0x00, 0xFF, 0xFF));
        // reverse: only R(35) → byte 17 high nibble
        let r = encode(0, -2, GearLedStyle::Progressive, levels());
        assert_eq!((r[17], r[18], r[19], r[20]), (0xF0, 0x00, 0x00, 0x00));
    }

    #[test]
    fn dim_backlight_and_clamp() {
        let r = encode(0, 0, GearLedStyle::Off, LedLevels { on: 99, off: 2 });
        assert_eq!(r[2], 0x22);
        let r = encode(
            EJECT_BIT,
            0,
            GearLedStyle::Off,
            LedLevels { on: 99, off: 2 },
        );
        assert_eq!(r[2], 0x2F);
    }
}
