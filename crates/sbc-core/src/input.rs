//! Raw 26-byte input report → [`SbcState`]. Layout: `docs/protocol.md`.

/// Length of one input report from the controller.
pub const INPUT_REPORT_LEN: usize = 26;
/// Number of physical buttons reported in the button bytes.
pub const BUTTON_COUNT: u32 = 39;
/// Mask of valid physical button bits.
pub const BUTTON_MASK: u64 = (1u64 << BUTTON_COUNT) - 1;
/// Raw bits of the 5 toggle switches (physical buttons 35–39).
pub const TOGGLE_MASK: u64 = 0b1_1111 << 34;

/// Decoded controller state. All values raw (unscaled).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SbcState {
    /// Bit `n` = physical button `n + 1` (see `docs/control-map.md`).
    pub buttons: u64,
    /// Aim stick X.
    pub aim_x: u16,
    /// Aim stick Y.
    pub aim_y: u16,
    /// Rotation lever.
    pub rotation: i16,
    /// Sight change stick X.
    pub sight_x: i16,
    /// Sight change stick Y.
    pub sight_y: i16,
    /// Clutch pedal.
    pub clutch: u16,
    /// Brake pedal.
    pub brake: u16,
    /// Throttle pedal.
    pub throttle: u16,
    /// Tuner dial position, 0..=15 (low nibble of byte 24).
    pub tuner: u8,
    /// Gear lever position.
    pub gear: i8,
}

/// Gear raw value → position index: R=0, N=1, 1st..5th = 2..6. `None` if unknown.
/// Raw values R=-2, N=-1, 1..5 confirmed on hardware; 0 appears for ~10 ms between gears.
#[must_use]
pub const fn gear_index(gear: i8) -> Option<u32> {
    match gear {
        -2 => Some(0),
        -1 => Some(1),
        1 => Some(2),
        2 => Some(3),
        3 => Some(4),
        4 => Some(5),
        5 => Some(6),
        _ => None,
    }
}

/// Reasons a report cannot be decoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// Fewer bytes than [`INPUT_REPORT_LEN`].
    TooShort {
        /// Bytes received.
        len: usize,
    },
}

/// Decode one input report. Extra trailing bytes are ignored.
///
/// # Errors
/// [`DecodeError::TooShort`] if `packet` is shorter than [`INPUT_REPORT_LEN`].
pub fn decode(packet: &[u8]) -> Result<SbcState, DecodeError> {
    let Some(r) = packet.first_chunk::<INPUT_REPORT_LEN>() else {
        return Err(DecodeError::TooShort { len: packet.len() });
    };
    let [
        _,
        _,
        b0,
        b1,
        b2,
        b3,
        b4,
        _,
        ax0,
        ax1,
        ay0,
        ay1,
        ro0,
        ro1,
        sx0,
        sx1,
        sy0,
        sy1,
        cl0,
        cl1,
        br0,
        br1,
        th0,
        th1,
        tuner,
        gear,
    ] = *r;

    let buttons = u64::from(b0)
        | (u64::from(b1) << 8)
        | (u64::from(b2) << 16)
        | (u64::from(b3) << 24)
        | (u64::from(b4 & 0x7F) << 32);

    let state = SbcState {
        buttons,
        aim_x: u16::from_le_bytes([ax0, ax1]),
        aim_y: u16::from_le_bytes([ay0, ay1]),
        rotation: i16::from_le_bytes([ro0, ro1]),
        sight_x: i16::from_le_bytes([sx0, sx1]),
        sight_y: i16::from_le_bytes([sy0, sy1]),
        clutch: u16::from_le_bytes([cl0, cl1]),
        brake: u16::from_le_bytes([br0, br1]),
        throttle: u16::from_le_bytes([th0, th1]),
        tuner: tuner & 0x0F, // upper nibble is not tuner data
        gear: i8::from_le_bytes([gear]),
    };
    debug_assert_eq!(state.buttons & !BUTTON_MASK, 0, "button bits beyond 39");
    debug_assert!(packet.len() >= INPUT_REPORT_LEN);
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet() -> [u8; INPUT_REPORT_LEN] {
        let mut p = [0u8; INPUT_REPORT_LEN];
        p[2] = 0b0001_0000; // CockpitHatch
        p[6] = 0xFF; // bit 7 must be dropped
        p[8..10].copy_from_slice(&0x1234u16.to_le_bytes());
        p[12..14].copy_from_slice(&(-200i16).to_le_bytes());
        p[22..24].copy_from_slice(&0xFFFFu16.to_le_bytes());
        p[24] = 0xA7; // upper nibble must be dropped
        p[25] = 0xFE; // -2 = reverse
        p
    }

    #[test]
    fn decodes_fields() {
        let s = decode(&packet()).unwrap();
        assert_eq!(s.buttons, (1 << 4) | (0x7F << 32));
        assert_eq!(s.aim_x, 0x1234);
        assert_eq!(s.rotation, -200);
        assert_eq!(s.throttle, 0xFFFF);
        assert_eq!(s.tuner, 7);
        assert_eq!(s.gear, -2);
    }

    #[test]
    fn rejects_short() {
        assert_eq!(decode(&[0u8; 25]), Err(DecodeError::TooShort { len: 25 }));
    }

    #[test]
    fn ignores_trailing_bytes() {
        let mut long = [0u8; 32];
        long[..INPUT_REPORT_LEN].copy_from_slice(&packet());
        assert_eq!(decode(&long), decode(&packet()));
    }
}
