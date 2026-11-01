//! `calibrate`: measure rest and travel of every axis, write `calibration.toml`.
//!
//! Phase 1 (hands off): rest position. Phase 2: move every axis to its limits.

use std::fmt::Write as _;
use std::path::Path;
use std::time::{Duration, Instant};

use sbc_core::calib::{AXIS_NAMES, SPRING_CENTERED, raw_axes};
use sbc_core::pad::AXIS_COUNT;
use sbc_core::{AxisCal, decode};
use sbc_usb::{MAX_REPORT, SbcDevice};

use crate::opts::Opts;

const REST_TIME: Duration = Duration::from_secs(3);
const READ_TIMEOUT: Duration = Duration::from_millis(100);
/// Pull each end in by span / this, so a full push reliably reaches the vJoy limit
/// and a released pedal reliably reads as zero.
const EDGE_MARGIN_DIV: i32 = 64;
/// Largest raw change allowed during the hands-off phase (sensor jitter is ~200).
const REST_MAX_SPREAD: i32 = 1024;

/// Per-axis measurements.
#[derive(Clone, Copy, Debug)]
struct Track {
    rest_sum: i64,
    rest_n: i64,
    rest_min: i32,
    rest_max: i32,
    min: i32,
    max: i32,
}

impl Track {
    const fn new() -> Self {
        Self {
            rest_sum: 0,
            rest_n: 0,
            rest_min: i32::MAX,
            rest_max: i32::MIN,
            min: i32::MAX,
            max: i32::MIN,
        }
    }

    fn add(&mut self, v: i32, resting: bool) {
        if resting {
            self.rest_sum = self.rest_sum.saturating_add(i64::from(v));
            self.rest_n = self.rest_n.saturating_add(1);
            self.rest_min = self.rest_min.min(v);
            self.rest_max = self.rest_max.max(v);
        }
        self.min = self.min.min(v);
        self.max = self.max.max(v);
    }

    /// Movement seen during the hands-off phase.
    const fn rest_spread(&self) -> i32 {
        self.rest_max.saturating_sub(self.rest_min)
    }

    fn rest_mean(&self) -> Option<i32> {
        let mean = self.rest_sum.checked_div(self.rest_n)?;
        i32::try_from(mean).ok()
    }

    /// Calibration for this axis, or `None` if it was not moved enough.
    fn result(&self, centered: bool) -> Option<AxisCal> {
        let margin = self.max.saturating_sub(self.min) / EDGE_MARGIN_DIV;
        let lo = self.min.saturating_add(margin);
        let hi = self.max.saturating_sub(margin);
        let cal = if centered {
            AxisCal::centered(lo, self.rest_mean()?, hi)
        } else {
            // Pedals: released = lowest value seen, whatever the rest phase saw.
            // Aim stick: no spring; its center is the middle of its travel.
            AxisCal::linear(lo, hi)
        };
        cal.validate().ok().map(|()| cal)
    }
}

pub fn run(opts: &Opts) -> Result<(), Box<dyn std::error::Error>> {
    let mut dev = SbcDevice::open()?;
    println!(
        "HANDS AND FEET OFF for {}s (rest position; leave the aim stick where it is) ...",
        REST_TIME.as_secs()
    );
    let tracks = measure(&mut dev, opts.run_time)?;
    let text = render(&tracks);
    print!("{text}");
    if let Some(out) = &opts.out_path {
        write_file(out, &text)?;
        println!("saved: {}", out.display());
    }
    Ok(())
}

fn measure(
    dev: &mut SbcDevice,
    move_time: Duration,
) -> Result<[Track; AXIS_COUNT], Box<dyn std::error::Error>> {
    let mut tracks = [Track::new(); AXIS_COUNT];
    let mut buf = [0u8; MAX_REPORT];
    let total = REST_TIME.saturating_add(move_time);
    let max_reads = total.as_millis().saturating_mul(2);
    let start = Instant::now();
    let mut announced = false;
    let mut reads = 0u128;
    while start.elapsed() < total && reads < max_reads {
        reads = reads.saturating_add(1);
        let resting = start.elapsed() < REST_TIME;
        if !resting && !announced {
            println!(
                "NOW MOVE for {}s: each stick and lever to all limits, slowly around the edges; \
                 each pedal to the floor.",
                move_time.as_secs()
            );
            announced = true;
        }
        let Some(n) = dev.read(&mut buf, READ_TIMEOUT)? else {
            continue;
        };
        let Ok(s) = decode(buf.get(..n).unwrap_or(&[])) else {
            continue;
        };
        for (t, v) in tracks.iter_mut().zip(raw_axes(&s)) {
            t.add(v, resting);
        }
    }
    debug_assert!(reads <= max_reads);
    if tracks.iter().any(|t| t.rest_n == 0) {
        return Err("no reports during the rest phase; is the controller connected?".into());
    }
    check_rest(&tracks)?;
    Ok(tracks)
}

/// Fail if anything moved while it should have been resting (a hand or foot on it).
fn check_rest(tracks: &[Track; AXIS_COUNT]) -> Result<(), String> {
    let moved: Vec<String> = AXIS_NAMES
        .iter()
        .zip(tracks.iter())
        .zip(SPRING_CENTERED)
        .filter(|((_, t), centered)| *centered && t.rest_spread() > REST_MAX_SPREAD)
        .map(|((n, t), _)| format!("{n} moved {}", t.rest_spread()))
        .collect();
    debug_assert!(moved.len() <= AXIS_COUNT);
    if moved.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "controls moved during HANDS OFF ({}); nothing saved, run again without touching the controller",
            moved.join(", ")
        ))
    }
}

fn render(tracks: &[Track; AXIS_COUNT]) -> String {
    let mut out = String::with_capacity(1024);
    let _ = writeln!(
        out,
        "# Axis calibration, generated by `sbc-capture calibrate`.\n\
         # Sticks/levers: min, center (rest), max. Pedals: min (released), max (floored).\n"
    );
    for ((name, t), centered) in AXIS_NAMES.iter().zip(tracks.iter()).zip(SPRING_CENTERED) {
        match t.result(centered) {
            Some(c) => {
                let _ = writeln!(out, "[{name}]\nmin = {}", c.min);
                if let Some(center) = c.center {
                    let _ = writeln!(out, "center = {center}");
                }
                let _ = writeln!(out, "max = {}\n", c.max);
            }
            None => {
                let _ = writeln!(
                    out,
                    "# [{name}] not moved enough (seen {}..{}); full raw range used\n",
                    t.min, t.max
                );
            }
        }
    }
    debug_assert!(out.starts_with("# Axis calibration"), "header first");
    out
}

fn write_file(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tracked(rest: &[i32], moving: &[i32]) -> Track {
        let mut t = Track::new();
        for v in rest {
            t.add(*v, true);
        }
        for v in moving {
            t.add(*v, false);
        }
        t
    }

    #[test]
    fn stick_uses_rest_mean_and_trimmed_edges() {
        let t = tracked(&[25_792, 25_856], &[0, 64_000]);
        let c = t.result(true).unwrap();
        assert_eq!(c.center, Some(25_824));
        assert_eq!(c.min, 1000);
        assert_eq!(c.max, 63_000);
    }

    #[test]
    fn pedal_min_is_lowest_value_plus_margin() {
        // A foot on the pedal during the rest phase must not shift the released point.
        let t = tracked(&[32_640, 32_700], &[4224, 64_000]);
        let c = t.result(false).unwrap();
        assert_eq!(c.center, None);
        assert_eq!(c.min, 4224 + (64_000 - 4224) / 64);
        assert!(c.max < 64_000);
    }

    #[test]
    fn stick_moved_during_rest_is_rejected() {
        let still = tracked(&[100, 228], &[-30_000, 30_000]);
        let bumped = tracked(&[100, 9000], &[-30_000, 30_000]);
        assert!(check_rest(&[still; AXIS_COUNT]).is_ok());
        let mut set = [still; AXIS_COUNT];
        set[1] = bumped; // aim_y: friction-held, may sit anywhere
        assert!(check_rest(&set).is_ok());
        set[3] = bumped; // sight_x: spring-centered
        let err = check_rest(&set).unwrap_err();
        assert!(err.contains("sight_x moved 8900"));
    }

    #[test]
    fn unmoved_axis_is_skipped() {
        let t = tracked(&[100, 120], &[110]);
        assert!(t.result(true).is_none());
        assert!(t.result(false).is_none());
        let text = render(&[t; AXIS_COUNT]);
        assert!(text.contains("# [aim_x] not moved enough"));
        assert!(!text.contains("\nmin ="));
    }

    #[test]
    fn rendered_file_round_trips_through_core_types() {
        let s = tracked(&[0], &[-30_000, 30_000]);
        let p = tracked(&[500], &[60_000]);
        let text = render(&[s, s, s, s, s, p, p, p]);
        assert_eq!(text.matches("center = ").count(), 3);
        assert!(text.contains("[aim_x]\nmin = "));
        assert!(!text.contains("[aim_x]\nmin = -29063\ncenter"));
        assert_eq!(text.matches("max = ").count(), AXIS_COUNT);
    }
}
