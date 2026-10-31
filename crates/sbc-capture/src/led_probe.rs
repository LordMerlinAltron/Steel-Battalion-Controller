//! `led-probe`: light one LED id at a time and print the expected control.
//! Watch the panel; any mismatch goes into docs/control-map.md.

use std::time::Duration;

use sbc_core::ledout::{GEAR_LED_BASE, LED_MAX_LEVEL, LED_REPORT_LEN, set_level};
use sbc_core::names::button_name;
use sbc_usb::{SbcDevice, UsbError};

use crate::opts::Opts;

const WRITE_TIMEOUT: Duration = Duration::from_millis(500);
const GEAR_NAMES: [&str; 7] = [
    "GearR", "GearN", "Gear1", "Gear2", "Gear3", "Gear4", "Gear5",
];

pub fn run(opts: &Opts) -> Result<(), UsbError> {
    let mut dev = SbcDevice::open()?;
    let len = opts.led_len.unwrap_or(LED_REPORT_LEN).min(LED_REPORT_LEN);
    let ids = u8::try_from(len.saturating_mul(2)).unwrap_or(u8::MAX);
    println!(
        "OUT {:02X?}  len {len}  walking LED ids 0..{ids}",
        dev.addrs().output
    );

    let mut report = [0u8; LED_REPORT_LEN];
    send(&mut dev, &report, len, "all off")?;
    for id in 0..ids {
        report.fill(0);
        set_level(&mut report, id, LED_MAX_LEVEL);
        let label = format!(
            "id {id:>2}  byte {:>2} {}  expect {}",
            id >> 1,
            nibble(id),
            expected(id)
        );
        send(&mut dev, &report, len, &label)?;
        std::thread::sleep(opts.dwell);
    }
    report.fill(0xFF);
    send(&mut dev, &report, len, "all on")?;
    std::thread::sleep(opts.dwell);
    report.fill(0);
    send(&mut dev, &report, len, "all off")?;
    Ok(())
}

const fn nibble(id: u8) -> &'static str {
    if id & 1 == 0 { "low " } else { "high" }
}

fn expected(id: u8) -> &'static str {
    match id {
        4..=33 => button_name(u32::from(id).saturating_sub(1)),
        _ if id >= GEAR_LED_BASE => GEAR_NAMES
            .get(usize::from(id.saturating_sub(GEAR_LED_BASE)))
            .copied()
            .unwrap_or("(none)"),
        _ => "(none)",
    }
}

fn send(
    dev: &mut SbcDevice,
    report: &[u8; LED_REPORT_LEN],
    len: usize,
    label: &str,
) -> Result<(), UsbError> {
    let data = report.get(..len).ok_or(UsbError::BadLength(len))?;
    dev.write(data, WRITE_TIMEOUT)?;
    println!("{label}");
    debug_assert!(len <= LED_REPORT_LEN);
    debug_assert!(!label.is_empty());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_names() {
        assert_eq!(expected(4), "Eject");
        assert_eq!(expected(33), "Comm5");
        assert_eq!(expected(35), "GearR");
        assert_eq!(expected(41), "Gear5");
        assert_eq!(expected(34), "(none)");
        assert_eq!(expected(42), "(none)");
        assert_eq!(expected(0), "(none)");
    }
}
