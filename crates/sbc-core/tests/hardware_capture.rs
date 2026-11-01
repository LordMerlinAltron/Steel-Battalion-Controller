//! Tests built from real controller captures (2026-11-01, `setup\4-capture.cmd`).
//! Packets: `logs/p1-raw-idle.txt` (hands off). Sequences: `logs/p1-watch.txt`.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use sbc_core::calib::{AXIS_MID, AxisCal, Calibration};
use sbc_core::pad::{AXIS_MIN, GEAR_BASE_BIT};
use sbc_core::{FilterTimes, InputFilter, PadMapper, SbcState, decode};

/// Distinct idle packets (sensor jitter only).
const IDLE: [[u8; 26]; 4] = [
    [
        0x00, 0x1A, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x65, 0x40, 0x71, 0x80, 0x00, 0x80,
        0x00, 0x80, 0x00, 0x80, 0x02, 0xC0, 0x10, 0x00, 0x00, 0x00, 0xFF,
    ],
    [
        0x00, 0x1A, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x65, 0x00, 0x71, 0x40, 0x00, 0x80,
        0x00, 0x80, 0x00, 0x40, 0x02, 0xC0, 0x10, 0x40, 0x00, 0x00, 0xFF,
    ],
    [
        0x00, 0x1A, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0xC0, 0x64, 0x40, 0x71, 0xC0, 0x00, 0x80,
        0x00, 0x80, 0x00, 0x40, 0x02, 0xC0, 0x10, 0x40, 0x00, 0x00, 0xFF,
    ],
    [
        0x00, 0x1A, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x65, 0x00, 0x71, 0x80, 0x00, 0x40,
        0x00, 0xC0, 0x00, 0x40, 0x02, 0x80, 0x10, 0x00, 0x00, 0x00, 0xFF,
    ],
];

/// Calibration measured on the same controller (pedal minimums from the hands-off capture).
fn measured() -> Calibration {
    Calibration {
        axes: [
            AxisCal::linear(1023, 64_449),
            AxisCal::linear(1023, 64_449),
            AxisCal::centered(-31_750, 128, 31_366),
            AxisCal::centered(-28_389, 128, 28_837),
            AxisCal::centered(-27_575, 160, 28_535),
            AxisCal::linear(64, 64_449),
            AxisCal::linear(704, 64_463),
            AxisCal::linear(4288, 64_506),
        ],
    }
}

#[test]
fn idle_packets_decode_to_rest_state() {
    for p in &IDLE {
        let s = decode(p).unwrap();
        assert_eq!(
            s.buttons, 0,
            "byte 6 bit 7 is set at idle and must be dropped"
        );
        assert_eq!(s.gear, -1, "lever was in N");
        assert_eq!(s.tuner, 0);
        assert!((25_792..=25_856).contains(&s.aim_x));
        assert!((28_928..=28_992).contains(&s.aim_y));
        assert!((4224..=4288).contains(&s.brake));
    }
}

#[test]
fn idle_maps_to_centered_levers_and_released_pedals() {
    let cal = measured();
    assert!(cal.validate().is_ok());
    let tol = 0x0100; // < 1 % of full range
    for p in &IDLE {
        let axes = cal.map(&decode(p).unwrap());
        // Rotation lever and sight stick spring back; the aim stick (axes 0, 1) stays put.
        for (i, a) in axes.iter().enumerate().take(5).skip(2) {
            assert!(
                a.abs_diff(AXIS_MID) <= tol,
                "lever axis {i} at rest: {a:#06x}"
            );
        }
        for (i, a) in axes.iter().skip(5).enumerate() {
            assert!(a.abs_diff(AXIS_MIN) <= tol, "pedal {i} at rest: {a:#06x}");
        }
    }
}

#[test]
fn gear_walk_from_watch_log() {
    // R, gap, N, gap, 1, gap, 2 ... 5 as logged; 0 = between gears.
    let walk: [(i8, Option<u32>); 13] = [
        (-2, Some(0)),
        (0, None),
        (-1, Some(1)),
        (0, None),
        (1, Some(2)),
        (0, None),
        (2, Some(3)),
        (0, None),
        (3, Some(4)),
        (0, None),
        (4, Some(5)),
        (0, None),
        (5, Some(6)),
    ];
    let mut m = PadMapper::new(60, measured());
    let gear_mask: u128 = 0x7F << GEAR_BASE_BIT;
    for (t, (raw, idx)) in (0u64..).zip(walk) {
        let s = SbcState {
            gear: raw,
            ..decode(&IDLE[0]).unwrap()
        };
        let got = m.map(&s, t.saturating_mul(10)).buttons & gear_mask;
        let want = idx.map_or(0, |i| 1u128 << (GEAR_BASE_BIT + i));
        assert_eq!(got, want, "gear raw {raw}");
    }
}

#[test]
fn toggle_bounce_from_watch_log_gives_one_on_and_one_off_pulse() {
    // ToggleFilterControl: + 46671, - 46679, + 46731 ... - 53319 (one flip on, one flip off).
    let sw: u64 = 1 << 34;
    let events: [(u64, bool); 5] = [
        (46_600, false),
        (46_671, true),
        (46_679, false),
        (46_731, true),
        (53_319, false),
    ];
    let on_pulse: u128 = 1 << 64; // vJoy 65
    let off_pulse: u128 = 1 << 69; // vJoy 70
    let mut f = InputFilter::new(FilterTimes::default());
    let mut m = PadMapper::new(60, measured());
    let mut ons = 0u32;
    let mut offs = 0u32;
    let mut prev = 0u128;
    let base = decode(&IDLE[0]).unwrap();
    // Feed one report every 4 ms, like the device.
    for t in (46_600u64..53_500).step_by(4) {
        let pressed = events
            .iter()
            .rev()
            .find(|(at, _)| *at <= t)
            .is_some_and(|e| e.1);
        let raw = SbcState {
            buttons: if pressed { sw } else { 0 },
            ..base
        };
        let pad = m.map(&f.apply(&raw, t), t);
        let rising = pad.buttons & !prev;
        ons += u32::from(rising & on_pulse != 0);
        offs += u32::from(rising & off_pulse != 0);
        prev = pad.buttons;
    }
    assert_eq!((ons, offs), (1, 1));
}
