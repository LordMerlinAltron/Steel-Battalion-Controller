//! vJoy feeder adapter: pushes [`sbc_core::PadState`] to a vJoy virtual joystick.
//!
//! `unsafe` is confined to the Windows FFI module (`win`). On other platforms
//! [`Vjoy::acquire`] returns [`VjoyError::Unsupported`] so the workspace still builds and tests.

#![deny(unsafe_code)]

pub mod position;
#[cfg(windows)]
#[allow(unsafe_code)]
mod win;

use std::fmt;
use std::path::{Path, PathBuf};

use sbc_core::PadState;

/// Highest vJoy device id.
pub const MAX_DEVICE_ID: u8 = 16;
/// HID usages of the 8 axes the bridge drives: X Y Z Rx Ry Rz Slider Dial.
pub const REQUIRED_AXES: [(u32, &str); 8] = [
    (0x30, "X"),
    (0x31, "Y"),
    (0x32, "Z"),
    (0x33, "Rx"),
    (0x34, "Ry"),
    (0x35, "Rz"),
    (0x36, "Slider"),
    (0x37, "Dial"),
];

/// Adapter errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VjoyError {
    /// Not running on Windows.
    Unsupported,
    /// Device id outside 1..=16.
    BadDeviceId(u8),
    /// `vJoyInterface.dll` could not be loaded.
    DllNotFound(String),
    /// DLL is missing an expected export.
    SymbolMissing(String),
    /// vJoy driver installed but disabled.
    DriverDisabled,
    /// Device not free (status code from `GetVJDStatus`).
    NotFree {
        /// Device id.
        id: u8,
        /// 0 own, 1 free, 2 busy, 3 missing, 4 unknown.
        status: i32,
    },
    /// `AcquireVJD` failed.
    AcquireFailed(u8),
    /// Device lacks a required axis.
    MissingAxis(&'static str),
    /// Device has too few buttons.
    TooFewButtons {
        /// Configured.
        have: i32,
        /// Needed.
        need: u32,
    },
    /// `UpdateVJD` returned failure.
    UpdateFailed,
}

impl fmt::Display for VjoyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => write!(f, "vJoy is only available on Windows"),
            Self::BadDeviceId(id) => write!(f, "vJoy device id {id} not in 1..={MAX_DEVICE_ID}"),
            Self::DllNotFound(p) => write!(f, "cannot load {p} (is vJoy installed?)"),
            Self::SymbolMissing(s) => write!(f, "vJoyInterface.dll lacks export {s}"),
            Self::DriverDisabled => write!(
                f,
                "vJoy driver is disabled (run 'Configure vJoy' and enable it)"
            ),
            Self::NotFree { id, status } => write!(
                f,
                "vJoy device {id} not free (status {status}: 2=busy by another feeder, 3=not configured)"
            ),
            Self::AcquireFailed(id) => write!(f, "AcquireVJD({id}) failed"),
            Self::MissingAxis(a) => write!(
                f,
                "vJoy device lacks axis {a} (enable it in 'Configure vJoy')"
            ),
            Self::TooFewButtons { have, need } => {
                write!(
                    f,
                    "vJoy device has {have} buttons, needs {need} (set it in 'Configure vJoy')"
                )
            }
            Self::UpdateFailed => write!(f, "UpdateVJD failed (device lost?)"),
        }
    }
}

impl std::error::Error for VjoyError {}

/// Default DLL location for the x64 vJoy install.
#[must_use]
pub fn default_dll_path() -> PathBuf {
    let base = std::env::var_os("ProgramFiles")
        .map_or_else(|| PathBuf::from(r"C:\Program Files"), PathBuf::from);
    base.join("vJoy").join("x64").join("vJoyInterface.dll")
}

/// An acquired vJoy device. Released (and reset to neutral) on drop.
pub struct Vjoy {
    id: u8,
    #[cfg(windows)]
    api: win::Api,
}

impl Vjoy {
    /// Load the DLL, check the device configuration, and acquire device `id`.
    ///
    /// # Errors
    /// Any [`VjoyError`] describing why the device cannot be used.
    pub fn acquire(id: u8, dll: &Path, need_buttons: u32) -> Result<Self, VjoyError> {
        if id == 0 || id > MAX_DEVICE_ID {
            return Err(VjoyError::BadDeviceId(id));
        }
        Self::acquire_platform(id, dll, need_buttons)
    }

    #[cfg(windows)]
    fn acquire_platform(id: u8, dll: &Path, need_buttons: u32) -> Result<Self, VjoyError> {
        let api = win::Api::load(dll)?;
        if !api.enabled() {
            return Err(VjoyError::DriverDisabled);
        }
        let status = api.status(id);
        if status != 0 && status != 1 {
            return Err(VjoyError::NotFree { id, status });
        }
        for (usage, name) in REQUIRED_AXES {
            if !api.axis_exists(id, usage) {
                return Err(VjoyError::MissingAxis(name));
            }
        }
        let have = api.button_count(id);
        if u32::try_from(have).unwrap_or(0) < need_buttons {
            return Err(VjoyError::TooFewButtons {
                have,
                need: need_buttons,
            });
        }
        if !api.acquire(id) {
            return Err(VjoyError::AcquireFailed(id));
        }
        let _ = api.reset(id);
        debug_assert!(id >= 1);
        Ok(Self { id, api })
    }

    #[cfg(not(windows))]
    fn acquire_platform(_id: u8, dll: &Path, need_buttons: u32) -> Result<Self, VjoyError> {
        debug_assert!(need_buttons <= 128);
        debug_assert!(!dll.as_os_str().is_empty());
        Err(VjoyError::Unsupported)
    }

    /// Device id in use.
    #[must_use]
    pub const fn id(&self) -> u8 {
        self.id
    }

    /// Push one pad state.
    ///
    /// # Errors
    /// [`VjoyError::UpdateFailed`] if the driver rejects the update.
    pub fn update(&self, pad: &PadState) -> Result<(), VjoyError> {
        let pos = position::from_pad(self.id, pad);
        debug_assert_eq!(pos.b_device, self.id);
        self.update_platform(&pos)
    }

    #[cfg(windows)]
    fn update_platform(&self, pos: &position::JoystickPositionV2) -> Result<(), VjoyError> {
        if self.api.update(self.id, pos) {
            Ok(())
        } else {
            Err(VjoyError::UpdateFailed)
        }
    }

    #[cfg(not(windows))]
    #[allow(clippy::unused_self)]
    const fn update_platform(&self, _pos: &position::JoystickPositionV2) -> Result<(), VjoyError> {
        Err(VjoyError::Unsupported)
    }
}

impl Drop for Vjoy {
    fn drop(&mut self) {
        let _ = self.update(&PadState::neutral());
        #[cfg(windows)]
        self.api.relinquish(self.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_ids() {
        let dll = default_dll_path();
        assert_eq!(
            Vjoy::acquire(0, &dll, 74).err(),
            Some(VjoyError::BadDeviceId(0))
        );
        assert_eq!(
            Vjoy::acquire(17, &dll, 74).err(),
            Some(VjoyError::BadDeviceId(17))
        );
    }

    #[test]
    fn default_dll_is_x64() {
        let p = default_dll_path();
        assert!(
            p.ends_with("vJoy/x64/vJoyInterface.dll") || p.ends_with(r"vJoy\x64\vJoyInterface.dll")
        );
    }
}
