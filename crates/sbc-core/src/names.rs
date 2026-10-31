//! Human names for the 39 raw button bits (index = bit). Source: 2019 driver, unverified.

use crate::input::BUTTON_COUNT;

/// Button names, index = raw bit, vJoy button = index + 1.
pub const BUTTON_NAMES: [&str; 39] = [
    "RightJoyMainWeapon",
    "RightJoyFire",
    "RightJoyLockOn",
    "Eject",
    "CockpitHatch",
    "Ignition",
    "Start",
    "MultiMonOpenClose",
    "MultiMonMapZoomInOut",
    "MultiMonModeSelect",
    "MultiMonSubMonitor",
    "MainMonZoomIn",
    "MainMonZoomOut",
    "FunctionFSS",
    "FunctionManipulator",
    "FunctionLineColorChange",
    "Washing",
    "Extinguisher",
    "Chaff",
    "FunctionTankDetach",
    "FunctionOverride",
    "FunctionNightScope",
    "FunctionF1",
    "FunctionF2",
    "FunctionF3",
    "WeaponCtrlMain",
    "WeaponCtrlSub",
    "WeaponCtrlMagazineChange",
    "Comm1",
    "Comm2",
    "Comm3",
    "Comm4",
    "Comm5",
    "LeftJoySightChange",
    "ToggleFilterControl",
    "ToggleOxygenSupply",
    "ToggleFuelFlowRate",
    "ToggleBufferMaterial",
    "ToggleVTLocation",
];

/// Name for raw bit `bit`, or `"?"` if out of range.
#[must_use]
pub fn button_name(bit: u32) -> &'static str {
    debug_assert_eq!(BUTTON_NAMES.len(), 39);
    usize::try_from(bit)
        .ok()
        .filter(|_| bit < BUTTON_COUNT)
        .and_then(|i| BUTTON_NAMES.get(i))
        .copied()
        .unwrap_or("?")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup() {
        assert_eq!(button_name(4), "CockpitHatch");
        assert_eq!(button_name(38), "ToggleVTLocation");
        assert_eq!(button_name(39), "?");
    }
}
