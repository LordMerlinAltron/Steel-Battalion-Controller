//! `JOYSTICK_POSITION_V2` layout (vJoy SDK `public.h`) and conversion from [`PadState`]. Pure.

use sbc_core::PadState;

/// Unused hat value (continuous POV neutral).
const HAT_NEUTRAL: u32 = u32::MAX;

/// Mirror of the SDK struct passed to `UpdateVJD`. Field order and `repr(C)` are load-bearing.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(clippy::struct_field_names)]
pub struct JoystickPositionV2 {
    /// 1-based device id.
    pub b_device: u8,
    /// Throttle (unused).
    pub w_throttle: i32,
    /// Rudder (unused).
    pub w_rudder: i32,
    /// Aileron (unused).
    pub w_aileron: i32,
    /// X.
    pub w_axis_x: i32,
    /// Y.
    pub w_axis_y: i32,
    /// Z.
    pub w_axis_z: i32,
    /// Rx.
    pub w_axis_x_rot: i32,
    /// Ry.
    pub w_axis_y_rot: i32,
    /// Rz.
    pub w_axis_z_rot: i32,
    /// Slider 0.
    pub w_slider: i32,
    /// Slider 1 (dial).
    pub w_dial: i32,
    /// Wheel (unused).
    pub w_wheel: i32,
    /// Unused velocity axes.
    pub w_axis_vx: i32,
    /// Unused.
    pub w_axis_vy: i32,
    /// Unused.
    pub w_axis_vz: i32,
    /// Unused.
    pub w_axis_vbrx: i32,
    /// Unused.
    pub w_axis_vbry: i32,
    /// Unused.
    pub w_axis_vbrz: i32,
    /// Buttons 1–32.
    pub l_buttons: i32,
    /// POV hats.
    pub b_hats: u32,
    /// POV hat 2.
    pub b_hats_ex1: u32,
    /// POV hat 3.
    pub b_hats_ex2: u32,
    /// POV hat 4.
    pub b_hats_ex3: u32,
    /// Buttons 33–64.
    pub l_buttons_ex1: i32,
    /// Buttons 65–96.
    pub l_buttons_ex2: i32,
    /// Buttons 97–128.
    pub l_buttons_ex3: i32,
}

/// Expected `sizeof(JOYSTICK_POSITION_V2)` on x64.
pub const POSITION_SIZE: usize = 108;

/// Bits `32*word .. 32*word+31` of `buttons` as the SDK's signed 32-bit word.
fn button_word(buttons: u128, word: u32) -> i32 {
    let shifted = buttons.checked_shr(word.saturating_mul(32)).unwrap_or(0);
    let low = u32::try_from(shifted & u128::from(u32::MAX)).unwrap_or(0);
    debug_assert!(word < 4, "vJoy has 4 button words");
    i32::from_ne_bytes(low.to_ne_bytes())
}

/// Build the SDK struct for `device` from a pad state.
#[must_use]
pub fn from_pad(device: u8, pad: &PadState) -> JoystickPositionV2 {
    let [x, y, z, rx, ry, rz, sl0, sl1] = pad.axes.map(i32::from);
    let pos = JoystickPositionV2 {
        b_device: device,
        w_axis_x: x,
        w_axis_y: y,
        w_axis_z: z,
        w_axis_x_rot: rx,
        w_axis_y_rot: ry,
        w_axis_z_rot: rz,
        w_slider: sl0,
        w_dial: sl1,
        l_buttons: button_word(pad.buttons, 0),
        l_buttons_ex1: button_word(pad.buttons, 1),
        l_buttons_ex2: button_word(pad.buttons, 2),
        l_buttons_ex3: button_word(pad.buttons, 3),
        b_hats: HAT_NEUTRAL,
        b_hats_ex1: HAT_NEUTRAL,
        b_hats_ex2: HAT_NEUTRAL,
        b_hats_ex3: HAT_NEUTRAL,
        ..JoystickPositionV2::default()
    };
    debug_assert_eq!(pos.b_device, device);
    debug_assert!(device >= 1, "vJoy device ids are 1-based");
    pos
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{offset_of, size_of};

    #[test]
    fn layout_matches_sdk() {
        assert_eq!(size_of::<JoystickPositionV2>(), POSITION_SIZE);
        assert_eq!(offset_of!(JoystickPositionV2, w_throttle), 4);
        assert_eq!(offset_of!(JoystickPositionV2, w_axis_x), 16);
        assert_eq!(offset_of!(JoystickPositionV2, w_dial), 44);
        assert_eq!(offset_of!(JoystickPositionV2, l_buttons), 76);
        assert_eq!(offset_of!(JoystickPositionV2, b_hats), 80);
        assert_eq!(offset_of!(JoystickPositionV2, l_buttons_ex1), 96);
        assert_eq!(offset_of!(JoystickPositionV2, l_buttons_ex3), 104);
    }

    #[test]
    fn buttons_split_into_words() {
        let pad = PadState {
            axes: [1, 2, 3, 4, 5, 6, 7, 8],
            buttons: 1 | (1 << 31) | (1 << 32) | (1 << 73),
        };
        let p = from_pad(1, &pad);
        assert_eq!(
            p.l_buttons,
            i32::from_ne_bytes(0x8000_0001u32.to_ne_bytes())
        );
        assert_eq!(p.l_buttons_ex1, 1);
        assert_eq!(p.l_buttons_ex2, 1 << 9);
        assert_eq!(p.l_buttons_ex3, 0);
        assert_eq!((p.w_axis_x, p.w_slider, p.w_dial), (1, 7, 8));
        assert_eq!(p.b_hats, HAT_NEUTRAL);
    }
}
