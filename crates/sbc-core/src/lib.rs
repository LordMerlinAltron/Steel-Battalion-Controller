//! Pure logic for the Steel Battalion controller bridge.
//!
//! No I/O, no allocation. Input: raw report bytes. Output: virtual joystick state and LED state.

pub mod input;
pub mod led;
pub mod ledout;
pub mod names;
pub mod pad;

pub use input::{DecodeError, INPUT_REPORT_LEN, SbcState, decode};
pub use led::{LedLatch, LedModes};
pub use ledout::{GearLedStyle, LED_REPORT_LEN, LedLevels, encode as encode_leds};
pub use pad::{PadMapper, PadState};
