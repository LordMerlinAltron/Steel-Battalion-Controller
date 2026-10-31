//! `sbc-bridge` — Steel Battalion controller → vJoy virtual joystick, with button LEDs.
//!
//! Waits for the controller, feeds vJoy while it is connected, and survives unplug/replug.

#![forbid(unsafe_code)]

mod config;
mod leds;
mod session;

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sbc_core::PadState;
use sbc_core::pad::PAD_BUTTON_COUNT;
use sbc_usb::{SbcDevice, UsbError};
use sbc_vjoy::Vjoy;

use config::Config;
use session::{Session, SessionEnd};

const USAGE: &str = "usage: sbc-bridge [--config FILE]";

/// Timestamped log line on stderr.
pub fn log(msg: &str) {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    eprintln!("[{secs}] {msg}");
}

fn main() -> ExitCode {
    let cfg = match load_config() {
        Ok(c) => c,
        Err(e) => {
            log(&e);
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    let vjoy = match Vjoy::acquire(cfg.vjoy.device_id, &cfg.dll_path(), PAD_BUTTON_COUNT) {
        Ok(v) => v,
        Err(e) => {
            log(&format!("vJoy: {e}"));
            return ExitCode::FAILURE;
        }
    };
    log(&format!("vJoy device {} acquired", vjoy.id()));
    let stop = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&stop);
    if let Err(e) = ctrlc::set_handler(move || flag.store(true, Ordering::Relaxed)) {
        log(&format!("Ctrl+C handler not installed: {e}"));
    }
    supervise(&vjoy, &cfg, &stop);
    log("stopped; vJoy reset to neutral");
    ExitCode::SUCCESS
}

fn load_config() -> Result<Config, String> {
    let args: Vec<String> = std::env::args().skip(1).take(4).collect();
    let mut candidates: Vec<PathBuf> = Vec::with_capacity(3);
    match args.as_slice() {
        [] => {}
        [flag, path] if flag == "--config" => candidates.push(PathBuf::from(path)),
        _ => return Err(format!("bad arguments: {args:?}")),
    }
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from))
    {
        candidates.push(dir.join("sbc-bridge.toml"));
    }
    candidates.push(PathBuf::from("config").join("default.toml"));
    let (cfg, source) = config::load(&candidates)?;
    log(&format!("config: {source}"));
    Ok(cfg)
}

/// Reconnect loop. Runs until Ctrl+C.
fn supervise(vjoy: &Vjoy, cfg: &Config, stop: &AtomicBool) {
    let pause = Duration::from_millis(cfg.usb.reconnect_ms);
    let mut waiting_logged = false;
    while !stop.load(Ordering::Relaxed) {
        match SbcDevice::open() {
            Ok(dev) => {
                waiting_logged = false;
                log(&format!("controller connected ({:02X?})", dev.addrs()));
                let end = Session::new(dev, vjoy, cfg).run(stop);
                report_end(&end);
                let _ = vjoy.update(&PadState::neutral());
            }
            Err(UsbError::NotFound) => {
                if !waiting_logged {
                    log("waiting for controller 0A7B:D000 ...");
                    waiting_logged = true;
                }
                std::thread::sleep(pause);
            }
            Err(e) => {
                log(&format!("open failed: {e}"));
                std::thread::sleep(pause);
            }
        }
    }
    debug_assert!(stop.load(Ordering::Relaxed));
}

fn report_end(end: &SessionEnd) {
    match end {
        SessionEnd::Stopped => {}
        SessionEnd::Usb(UsbError::Disconnected) => log("controller disconnected"),
        SessionEnd::Usb(e) => log(&format!("USB error: {e}; reconnecting")),
        SessionEnd::Vjoy(e) => log(&format!("vJoy error: {e}; reconnecting")),
        SessionEnd::BadPackets => log("too many malformed packets; reconnecting"),
    }
}
