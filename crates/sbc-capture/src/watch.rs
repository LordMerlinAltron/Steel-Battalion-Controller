//! `watch`: print every control change with its name. Used to verify docs/control-map.md.

use std::time::{Duration, Instant};

use sbc_core::input::BUTTON_COUNT;
use sbc_core::names::button_name;
use sbc_core::{SbcState, decode};
use sbc_usb::{MAX_REPORT, SbcDevice, UsbError};

use crate::opts::Opts;

/// Minimum raw change before an axis is printed (filters jitter).
const AXIS_STEP: i32 = 2048;
const READ_TIMEOUT: Duration = Duration::from_millis(100);

pub fn run(opts: &Opts) -> Result<(), UsbError> {
    let mut dev = SbcDevice::open()?;
    let mut buf = [0u8; MAX_REPORT];
    let mut prev: Option<SbcState> = None;
    let start = Instant::now();
    println!(
        "watching for {}s — press each control once",
        opts.run_time.as_secs()
    );

    let max_reads = opts.run_time.as_millis().saturating_mul(2);
    let mut reads = 0u128;
    while start.elapsed() < opts.run_time && reads < max_reads {
        reads = reads.saturating_add(1);
        let Some(n) = dev.read(&mut buf, READ_TIMEOUT)? else {
            continue;
        };
        let Ok(cur) = decode(buf.get(..n).unwrap_or(&[])) else {
            println!("short packet: {n} bytes");
            continue;
        };
        let t = start.elapsed().as_millis();
        match prev {
            None => println!("{t:>7} ms  first report: {cur:?}"),
            Some(p) => print_changes(t, &p, &cur),
        }
        prev = Some(cur);
    }
    debug_assert!(reads <= max_reads);
    Ok(())
}

fn print_changes(t: u128, p: &SbcState, c: &SbcState) {
    let changed = p.buttons ^ c.buttons;
    for bit in 0..BUTTON_COUNT {
        let mask = 1u64 << bit;
        if changed & mask != 0 {
            let sign = if c.buttons & mask != 0 { '+' } else { '-' };
            println!(
                "{t:>7} ms  {sign} vJoy {:>2}  B{}.{}  {}",
                bit.saturating_add(1),
                bit / 8,
                bit % 8,
                button_name(bit)
            );
        }
    }
    let axes: [(&str, i32, i32); 8] = [
        ("AimX", p.aim_x.into(), c.aim_x.into()),
        ("AimY", p.aim_y.into(), c.aim_y.into()),
        ("Rotation", p.rotation.into(), c.rotation.into()),
        ("SightX", p.sight_x.into(), c.sight_x.into()),
        ("SightY", p.sight_y.into(), c.sight_y.into()),
        ("Clutch", p.clutch.into(), c.clutch.into()),
        ("Brake", p.brake.into(), c.brake.into()),
        ("Throttle", p.throttle.into(), c.throttle.into()),
    ];
    for (name, a, b) in axes {
        if a.abs_diff(b) >= AXIS_STEP.unsigned_abs() {
            println!("{t:>7} ms  ~ {name:<8} {b}");
        }
    }
    if p.gear != c.gear {
        println!("{t:>7} ms  ~ Gear     raw {}", c.gear);
    }
    if p.tuner != c.tuner {
        println!("{t:>7} ms  ~ Tuner    raw {}", c.tuner);
    }
}
