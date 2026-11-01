//! `sbc-bridge` — Steel Battalion controller → vJoy virtual joystick, with button LEDs.
//!
//! Waits for vJoy and the controller, feeds vJoy while the controller is connected,
//! and survives unplug/replug. Release builds on Windows run without a console window;
//! use `--log FILE` to see what it does. Stop it by creating `sbc-bridge.stop` next to the exe.

#![forbid(unsafe_code)]
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod calibration;
mod config;
mod leds;
mod runtime;
mod session;

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::Ordering;
use std::time::Duration;

use sbc_core::PadState;
use sbc_core::pad::PAD_BUTTON_COUNT;
use sbc_usb::{SbcDevice, UsbError};
use sbc_vjoy::Vjoy;

use config::Config;
use runtime::{Args, InstanceLock, Logger, Stop, exe_dir};
use session::{Session, SessionEnd};

/// Retry vJoy every this long while it is unavailable (e.g. early at logon).
const VJOY_RETRY: Duration = Duration::from_secs(5);
/// Log a still-failing vJoy every N retries (~5 min).
const VJOY_LOG_EVERY: u32 = 60;
/// Sleep slice so stop requests are seen promptly while waiting.
const WAIT_SLICE: Duration = Duration::from_millis(100);

fn main() -> ExitCode {
    let args = match Args::parse(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}\nusage: sbc-bridge [--config FILE] [--log FILE]");
            return ExitCode::from(2);
        }
    };
    let log = Logger::open(args.log.as_deref());
    let dir = exe_dir();
    let Some(_lock) = InstanceLock::acquire(&dir.join("sbc-bridge.lock")) else {
        log.line("another sbc-bridge is already running; exiting");
        return ExitCode::FAILURE;
    };
    let cfg = match load_config(&args, &dir, &log) {
        Ok(c) => c,
        Err(e) => {
            log.line(&e);
            return ExitCode::from(2);
        }
    };
    let stop = Stop::new(dir.join("sbc-bridge.stop"));
    let flag = stop.flag();
    if let Err(e) = ctrlc::set_handler(move || flag.store(true, Ordering::Relaxed)) {
        log.line(&format!("Ctrl+C handler not installed: {e}"));
    }
    log.line(&format!("sbc-bridge {} started", env!("CARGO_PKG_VERSION")));
    if let Some(vjoy) = acquire_vjoy(&cfg, &log, &stop) {
        log.line(&format!("vJoy device {} acquired", vjoy.id()));
        supervise(&vjoy, &cfg, &log, &stop);
    }
    log.line("stopped; vJoy reset to neutral");
    ExitCode::SUCCESS
}

fn load_config(args: &Args, dir: &std::path::Path, log: &Logger) -> Result<Config, String> {
    let mut candidates: Vec<PathBuf> = Vec::with_capacity(3);
    candidates.extend(args.config.clone());
    candidates.push(dir.join("sbc-bridge.toml"));
    candidates.push(PathBuf::from("config").join("default.toml"));
    let (mut cfg, source) = config::load(&candidates)?;
    log.line(&format!("config: {source}"));
    debug_assert!(candidates.len() <= 3);
    let cal_paths = [
        dir.join("sbc-calibration.toml"),
        PathBuf::from("config").join("calibration.toml"),
    ];
    for path in &cal_paths {
        if let Some(cal) = calibration::load(path)? {
            log.line(&format!("calibration: {}", path.display()));
            cfg.calibration = cal;
            return Ok(cfg);
        }
    }
    log.line("calibration: none (full raw range)");
    debug_assert_eq!(cfg.calibration, sbc_core::Calibration::full_range());
    Ok(cfg)
}

/// Keep trying until vJoy is usable or a stop is requested.
fn acquire_vjoy(cfg: &Config, log: &Logger, stop: &Stop) -> Option<Vjoy> {
    let dll = cfg.dll_path();
    let mut attempt: u32 = 0;
    while !stop.requested() {
        match Vjoy::acquire(cfg.vjoy.device_id, &dll, PAD_BUTTON_COUNT) {
            Ok(v) => return Some(v),
            Err(e) => {
                if attempt.is_multiple_of(VJOY_LOG_EVERY) {
                    log.line(&format!(
                        "vJoy: {e}; retrying every {}s",
                        VJOY_RETRY.as_secs()
                    ));
                }
                attempt = attempt.saturating_add(1);
                wait(VJOY_RETRY, stop);
            }
        }
    }
    None
}

/// Sleep up to `d`, waking early on a stop request.
fn wait(d: Duration, stop: &Stop) {
    let slices = d
        .as_millis()
        .checked_div(WAIT_SLICE.as_millis())
        .unwrap_or(1);
    for _ in 0..slices {
        if stop.requested() {
            return;
        }
        std::thread::sleep(WAIT_SLICE);
    }
}

/// Reconnect loop. Runs until a stop is requested.
fn supervise(vjoy: &Vjoy, cfg: &Config, log: &Logger, stop: &Stop) {
    let pause = Duration::from_millis(cfg.usb.reconnect_ms);
    let mut waiting_logged = false;
    while !stop.requested() {
        match SbcDevice::open() {
            Ok(dev) => {
                waiting_logged = false;
                log.line(&format!("controller connected ({:02X?})", dev.addrs()));
                let end = Session::new(dev, vjoy, cfg, log).run(stop);
                report_end(&end, log);
                let _ = vjoy.update(&PadState::neutral());
            }
            Err(UsbError::NotFound) => {
                if !waiting_logged {
                    log.line("waiting for controller 0A7B:D000 ...");
                    waiting_logged = true;
                }
                wait(pause, stop);
            }
            Err(e) => {
                log.line(&format!("open failed: {e}"));
                wait(pause, stop);
            }
        }
    }
}

fn report_end(end: &SessionEnd, log: &Logger) {
    match end {
        SessionEnd::Stopped => {}
        SessionEnd::Usb(UsbError::Disconnected) => log.line("controller disconnected"),
        SessionEnd::Usb(e) => log.line(&format!("USB error: {e}; reconnecting")),
        SessionEnd::Vjoy(e) => log.line(&format!("vJoy error: {e}; reconnecting")),
        SessionEnd::BadPackets => log.line("too many malformed packets; reconnecting"),
    }
}
