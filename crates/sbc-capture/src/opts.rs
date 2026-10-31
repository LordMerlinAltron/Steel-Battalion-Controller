//! Minimal argument parsing (no dependencies).

use std::path::PathBuf;
use std::time::Duration;

const MAX_ARGS: usize = 32;

/// Subcommand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Help,
    Info,
    Watch,
    Raw,
    LedProbe,
}

/// Parsed options with defaults applied.
#[derive(Clone, Debug)]
pub struct Opts {
    pub command: Command,
    pub run_time: Duration,
    pub save_dir: Option<PathBuf>,
    pub max_packets: usize,
    pub led_len: Option<usize>,
    pub dwell: Duration,
}

impl Opts {
    pub fn parse(args: impl Iterator<Item = String>) -> Result<Self, String> {
        let args: Vec<String> = args.take(MAX_ARGS).collect();
        let mut it = args.iter();
        let command = match it.next().map(String::as_str) {
            None | Some("help" | "-h" | "--help") => Command::Help,
            Some("info") => Command::Info,
            Some("watch") => Command::Watch,
            Some("raw") => Command::Raw,
            Some("led-probe") => Command::LedProbe,
            Some(other) => return Err(format!("unknown command '{other}'")),
        };
        let mut o = Self {
            command,
            run_time: Duration::from_secs(120),
            save_dir: None,
            max_packets: 500,
            led_len: None,
            dwell: Duration::from_millis(1500),
        };
        while let Some(flag) = it.next() {
            let val = it.next().ok_or_else(|| format!("{flag} needs a value"))?;
            o.apply(flag, val)?;
        }
        debug_assert!(o.max_packets > 0);
        Ok(o)
    }

    fn apply(&mut self, flag: &str, val: &str) -> Result<(), String> {
        match flag {
            "--secs" => self.run_time = Duration::from_secs(num(flag, val, 1, 3600)?),
            "--save" => self.save_dir = Some(PathBuf::from(val)),
            "--max" => self.max_packets = usize_num(flag, val, 1, 100_000)?,
            "--len" => self.led_len = Some(usize_num(flag, val, 1, sbc_core::LED_REPORT_LEN)?),
            "--dwell" => self.dwell = Duration::from_millis(num(flag, val, 100, 10_000)?),
            _ => return Err(format!("unknown option '{flag}'")),
        }
        Ok(())
    }
}

fn num(flag: &str, val: &str, min: u64, max: u64) -> Result<u64, String> {
    let n: u64 = val
        .parse()
        .map_err(|_| format!("{flag}: '{val}' is not a number"))?;
    if (min..=max).contains(&n) {
        Ok(n)
    } else {
        Err(format!("{flag}: {n} not in {min}..={max}"))
    }
}

fn usize_num(flag: &str, val: &str, min: usize, max: usize) -> Result<usize, String> {
    let n: usize = val
        .parse()
        .map_err(|_| format!("{flag}: '{val}' is not a number"))?;
    if (min..=max).contains(&n) {
        Ok(n)
    } else {
        Err(format!("{flag}: {n} not in {min}..={max}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Result<Opts, String> {
        Opts::parse(s.split_whitespace().map(String::from))
    }

    #[test]
    fn defaults() {
        let o = parse("watch").unwrap();
        assert_eq!(o.command, Command::Watch);
        assert_eq!(o.run_time, Duration::from_secs(120));
    }

    #[test]
    fn led_options() {
        let o = parse("led-probe --len 22 --dwell 500").unwrap();
        assert_eq!(o.led_len, Some(22));
        assert_eq!(o.dwell, Duration::from_millis(500));
    }

    #[test]
    fn rejects_bad_input() {
        assert!(parse("dance").is_err());
        assert!(parse("watch --secs").is_err());
        assert!(parse("watch --secs 0").is_err());
        assert!(parse("led-probe --len 99").is_err());
    }
}
