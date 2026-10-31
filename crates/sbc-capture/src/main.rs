//! `sbc-capture` — Phase 1 evidence tool for the Steel Battalion controller.

#![forbid(unsafe_code)]

mod info;
mod led_probe;
mod opts;
mod raw;
mod watch;

use std::process::ExitCode;

use opts::{Command, Opts};

const USAGE: &str = "\
usage: sbc-capture <command> [options]

commands:
  info                      USB descriptors and endpoint addresses
  watch                     live named controls (press every button)
  raw                       raw input packets as hex
  led-probe                 light each LED id in turn, print expected control

options:
  --secs N        run time for watch/raw (default 120)
  --save DIR      raw: write each packet to DIR/pkt_NNNN.bin
  --max N         raw: stop after N packets (default 500)
  --len N         led-probe: report length (default 34)
  --dwell MS      led-probe: time per step (default 1500)";

fn main() -> ExitCode {
    let opts = match Opts::parse(std::env::args().skip(1)) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: {e}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let result: Result<(), Box<dyn std::error::Error>> = match opts.command {
        Command::Help => {
            println!("{USAGE}");
            Ok(())
        }
        Command::Info => info::run().map_err(Into::into),
        Command::Watch => watch::run(&opts).map_err(Into::into),
        Command::Raw => raw::run(&opts),
        Command::LedProbe => led_probe::run(&opts).map_err(Into::into),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
