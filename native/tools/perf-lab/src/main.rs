#![forbid(unsafe_code)]

//! Repeatable, first-party native transport measurements.
//!
//! The tool reports either in-process request latency for the owned wire,
//! transport, and core layers, or an actual local Windows named-pipe loopback.
//! It does not measure process startup, rendering, memory, or an Electron
//! application; those require separately documented, equivalent workloads
//! before comparison claims are made.

mod arguments;
mod report;
mod workload;

use std::process::ExitCode;

use arguments::Options;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("anodrel-perf-lab: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), String> {
    let options = Options::parse(std::env::args().skip(1))?;
    let report = workload::measure(options.workload, options.iterations)?;
    print!("{}", report.to_json());
    Ok(())
}
