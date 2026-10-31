//! One connected-controller session: USB read → decode → vJoy, plus LED writes.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use sbc_core::{PadMapper, SbcState, decode};
use sbc_usb::{MAX_REPORT, SbcDevice, UsbError};
use sbc_vjoy::{Vjoy, VjoyError};

use crate::config::Config;
use crate::leds::LedDriver;
use crate::log;

const LED_WRITE_TIMEOUT: Duration = Duration::from_millis(200);
/// Consecutive short/undecodable packets before the session gives up.
const MAX_BAD_PACKETS: u32 = 1000;

/// Why a session ended.
#[derive(Debug)]
pub enum SessionEnd {
    /// Shutdown requested.
    Stopped,
    /// USB error (unplug, stall, ...).
    Usb(UsbError),
    /// vJoy rejected an update.
    Vjoy(VjoyError),
    /// Device kept sending garbage.
    BadPackets,
}

/// Session state. Everything sized at construction; the loop does not allocate.
pub struct Session<'a> {
    dev: SbcDevice,
    vjoy: &'a Vjoy,
    timeout: Duration,
    mapper: PadMapper,
    leds: Option<LedDriver>,
    last: Option<SbcState>,
    bad: u32,
    t0: Instant,
}

impl<'a> Session<'a> {
    /// Wrap an opened device.
    pub fn new(dev: SbcDevice, vjoy: &'a Vjoy, cfg: &Config) -> Self {
        let leds_ok = cfg.leds.enabled && check_xid(&dev);
        let leds = leds_ok.then(|| {
            LedDriver::new(
                cfg.led_modes(),
                cfg.led_levels(),
                cfg.gear_style(),
                cfg.leds.clear_combo_ms,
            )
        });
        let mut s = Self {
            dev,
            vjoy,
            timeout: Duration::from_millis(cfg.usb.read_timeout_ms),
            mapper: PadMapper::new(cfg.vjoy.pulse_ms),
            leds,
            last: None,
            bad: 0,
            t0: Instant::now(),
        };
        s.send_connect_leds(cfg.leds.reset_on_connect);
        debug_assert!(s.timeout > Duration::ZERO);
        s
    }

    /// Run until stop, error, or unplug.
    pub fn run(&mut self, stop: &AtomicBool) -> SessionEnd {
        let mut buf = [0u8; MAX_REPORT];
        while !stop.load(Ordering::Relaxed) {
            if let Err(end) = self.step(&mut buf) {
                return end;
            }
        }
        SessionEnd::Stopped
    }

    fn step(&mut self, buf: &mut [u8; MAX_REPORT]) -> Result<(), SessionEnd> {
        let read = self.dev.read(buf, self.timeout).map_err(SessionEnd::Usb)?;
        let state = match read {
            None => self.last, // timeout: re-map last state so pulses expire
            Some(n) => {
                let decoded = decode(buf.get(..n).unwrap_or(&[])).ok();
                self.bad = if decoded.is_some() {
                    0
                } else {
                    self.bad.saturating_add(1)
                };
                if self.bad >= MAX_BAD_PACKETS {
                    return Err(SessionEnd::BadPackets);
                }
                decoded
            }
        };
        let Some(s) = state else { return Ok(()) };
        self.apply(&s)
    }

    fn apply(&mut self, s: &SbcState) -> Result<(), SessionEnd> {
        let now = self.now_ms();
        let pad = self.mapper.map(s, now);
        self.vjoy.update(&pad).map_err(SessionEnd::Vjoy)?;
        self.last = Some(*s);
        let report = self.leds.as_mut().and_then(|d| d.next(s, now));
        if let Some(r) = report {
            self.write_leds(&r);
        }
        debug_assert!(self.last.is_some());
        Ok(())
    }

    fn send_connect_leds(&mut self, reset: bool) {
        let report = self.leds.as_mut().map(|d| d.connect_report(reset));
        if let Some(r) = report {
            self.write_leds(&r);
        }
    }

    /// LED failures are not fatal: log once and disable LEDs for this session.
    fn write_leds(&mut self, report: &[u8]) {
        if let Err(e) = self.dev.write(report, LED_WRITE_TIMEOUT) {
            log(&format!(
                "LED write failed ({e}); LEDs disabled until reconnect"
            ));
            self.leds = None;
        }
        debug_assert!(report.len() <= sbc_usb::MAX_REPORT);
    }

    fn now_ms(&self) -> u64 {
        u64::try_from(self.t0.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}

/// Log the XID descriptor; LEDs stay enabled unless the device reports a different output size.
fn check_xid(dev: &SbcDevice) -> bool {
    let raw = match dev.xid_descriptor() {
        Ok(r) => r,
        Err(e) => {
            log(&format!(
                "XID descriptor unavailable ({e}); assuming 22-byte LED report"
            ));
            return true;
        }
    };
    match sbc_core::xid::parse(&raw) {
        Ok(x) if usize::from(x.max_output) == sbc_core::LED_REPORT_LEN => {
            log(&format!(
                "XID type {:02X}/{:02X}, in {} out {}",
                x.device_type, x.sub_type, x.max_input, x.max_output
            ));
            true
        }
        Ok(x) => {
            log(&format!(
                "XID output report is {} bytes, expected {}; LEDs disabled",
                x.max_output,
                sbc_core::LED_REPORT_LEN
            ));
            false
        }
        Err(e) => {
            log(&format!(
                "XID descriptor unreadable ({e:?}); assuming 22-byte LED report"
            ));
            true
        }
    }
}
