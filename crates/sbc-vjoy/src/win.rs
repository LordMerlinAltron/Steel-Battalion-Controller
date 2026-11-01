//! Windows FFI to `vJoyInterface.dll`. The only `unsafe` code in the workspace.
//! The DLL is loaded at runtime so the build needs no vJoy SDK or import library.

use std::ffi::c_void;
use std::mem::{size_of, size_of_val, transmute_copy};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows_sys::Win32::Foundation::{FARPROC, FreeLibrary, HMODULE};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

use crate::VjoyError;
use crate::position::JoystickPositionV2;

type FnVoidBool = unsafe extern "C" fn() -> i32;
type FnId = unsafe extern "C" fn(u32) -> i32;
type FnIdVoid = unsafe extern "C" fn(u32);
type FnUpdate = unsafe extern "C" fn(u32, *mut c_void) -> i32;
type FnAxis = unsafe extern "C" fn(u32, u32) -> i32;

/// Resolved DLL entry points.
pub struct Api {
    module: HMODULE,
    enabled: FnVoidBool,
    status: FnId,
    acquire: FnId,
    relinquish: FnIdVoid,
    update: FnUpdate,
    reset: FnId,
    button_count: FnId,
    axis_exists: FnAxis,
}

/// Resolve one export. `name` must be NUL-terminated.
///
/// # Safety
/// `T` must be the exact function-pointer type of the export.
unsafe fn sym<T: Copy>(module: HMODULE, name: &'static [u8]) -> Result<T, VjoyError> {
    debug_assert_eq!(name.last(), Some(&0), "symbol name must be NUL-terminated");
    debug_assert_eq!(size_of::<T>(), size_of::<FARPROC>());
    // SAFETY: module is a live handle from LoadLibraryW; name is NUL-terminated.
    let proc: FARPROC = unsafe { GetProcAddress(module, name.as_ptr()) };
    let Some(f) = proc else {
        let text = String::from_utf8_lossy(name.split_last().map_or(name, |(_, s)| s)).into_owned();
        return Err(VjoyError::SymbolMissing(text));
    };
    debug_assert_eq!(size_of_val(&f), size_of::<T>());
    // SAFETY: caller guarantees T is the export's real signature; sizes checked above.
    Ok(unsafe { transmute_copy::<_, T>(&f) })
}

impl Api {
    /// Load the DLL and resolve all exports used.
    pub fn load(dll: &Path) -> Result<Self, VjoyError> {
        let wide: Vec<u16> = dll
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: wide is NUL-terminated and outlives the call.
        let module = unsafe { LoadLibraryW(wide.as_ptr()) };
        if module.is_null() {
            return Err(VjoyError::DllNotFound(dll.display().to_string()));
        }
        // SAFETY: each type matches the vJoy SDK declaration (BOOL=i32, UINT=u32, cdecl=C on x64).
        let api = unsafe { Self::resolve(module) };
        if api.is_err() {
            // SAFETY: module came from LoadLibraryW and is not used after this.
            let _ = unsafe { FreeLibrary(module) };
        }
        api
    }

    unsafe fn resolve(module: HMODULE) -> Result<Self, VjoyError> {
        // SAFETY: forwarded from caller.
        unsafe {
            Ok(Self {
                module,
                enabled: sym(module, b"vJoyEnabled\0")?,
                status: sym(module, b"GetVJDStatus\0")?,
                acquire: sym(module, b"AcquireVJD\0")?,
                relinquish: sym(module, b"RelinquishVJD\0")?,
                update: sym(module, b"UpdateVJD\0")?,
                reset: sym(module, b"ResetVJD\0")?,
                button_count: sym(module, b"GetVJDButtonNumber\0")?,
                axis_exists: sym(module, b"GetVJDAxisExist\0")?,
            })
        }
    }

    pub fn enabled(&self) -> bool {
        // SAFETY: resolved export, no arguments.
        unsafe { (self.enabled)() != 0 }
    }

    /// `VjdStat`: 0 own, 1 free, 2 busy, 3 missing, 4 unknown.
    pub fn status(&self, id: u8) -> i32 {
        // SAFETY: resolved export, plain integer argument.
        unsafe { (self.status)(u32::from(id)) }
    }

    pub fn acquire(&self, id: u8) -> bool {
        // SAFETY: resolved export, plain integer argument.
        unsafe { (self.acquire)(u32::from(id)) != 0 }
    }

    pub fn relinquish(&self, id: u8) {
        // SAFETY: resolved export, plain integer argument.
        unsafe { (self.relinquish)(u32::from(id)) }
    }

    pub fn reset(&self, id: u8) -> bool {
        // SAFETY: resolved export, plain integer argument.
        unsafe { (self.reset)(u32::from(id)) != 0 }
    }

    pub fn button_count(&self, id: u8) -> i32 {
        // SAFETY: resolved export, plain integer argument.
        unsafe { (self.button_count)(u32::from(id)) }
    }

    pub fn axis_exists(&self, id: u8, usage: u32) -> bool {
        // SAFETY: resolved export, plain integer arguments.
        unsafe { (self.axis_exists)(u32::from(id), usage) != 0 }
    }

    pub fn update(&self, id: u8, pos: &JoystickPositionV2) -> bool {
        let mut copy = *pos;
        let ptr: *mut JoystickPositionV2 = &raw mut copy;
        // SAFETY: ptr is a valid, exclusive, correctly laid-out struct for the call's duration.
        unsafe { (self.update)(u32::from(id), ptr.cast::<c_void>()) != 0 }
    }
}

impl Drop for Api {
    fn drop(&mut self) {
        // SAFETY: module from LoadLibraryW; no function pointers are used after drop.
        let _ = unsafe { FreeLibrary(self.module) };
    }
}
