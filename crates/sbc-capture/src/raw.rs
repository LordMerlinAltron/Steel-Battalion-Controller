//! `raw`: hex dump of input packets; optional fixture files for sbc-core tests.

use std::fmt::Write as _;
use std::path::Path;
use std::time::{Duration, Instant};

use sbc_usb::{MAX_REPORT, SbcDevice};

use crate::opts::Opts;

const READ_TIMEOUT: Duration = Duration::from_millis(100);

pub fn run(opts: &Opts) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(dir) = &opts.save_dir {
        std::fs::create_dir_all(dir)?;
    }
    let mut dev = SbcDevice::open()?;
    let mut buf = [0u8; MAX_REPORT];
    let mut line = String::with_capacity(MAX_REPORT.saturating_mul(3));
    let start = Instant::now();
    let mut count = 0usize;

    while count < opts.max_packets && start.elapsed() < opts.run_time {
        let Some(n) = dev.read(&mut buf, READ_TIMEOUT)? else {
            continue;
        };
        let pkt = buf.get(..n).unwrap_or(&[]);
        line.clear();
        for b in pkt {
            write!(line, "{b:02X} ")?;
        }
        println!("{:>7} ms  [{n:>2}]  {line}", start.elapsed().as_millis());
        if let Some(dir) = &opts.save_dir {
            save(dir, count, pkt)?;
        }
        count = count.saturating_add(1);
    }
    debug_assert!(count <= opts.max_packets);
    println!("{count} packets");
    Ok(())
}

fn save(dir: &Path, index: usize, pkt: &[u8]) -> std::io::Result<()> {
    let path = dir.join(format!("pkt_{index:04}.bin"));
    std::fs::write(&path, pkt)?;
    debug_assert!(pkt.len() <= MAX_REPORT);
    Ok(())
}
