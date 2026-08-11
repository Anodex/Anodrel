//! Command-line options that keep a local benchmark bounded and repeatable.

pub const DEFAULT_ITERATIONS: usize = 5_000;
const MIN_ITERATIONS: usize = 10;
const MAX_ITERATIONS: usize = 100_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Workload {
    InProcess,
    WindowsPipe,
    Renderer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Options {
    pub iterations: usize,
    pub workload: Workload,
}

impl Options {
    pub fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut iterations = DEFAULT_ITERATIONS;
        let mut iterations_seen = false;
        let mut workload = Workload::InProcess;
        // One flag per workload, but at most one workload: measuring two things
        // in one run would report them under a single benchmark identifier.
        let mut workload_seen = false;
        let mut arguments = arguments.into_iter();
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--iterations" => {
                    if iterations_seen {
                        return Err("--iterations may be supplied only once".to_owned());
                    }
                    let value = arguments
                        .next()
                        .ok_or_else(|| "--iterations requires a whole number".to_owned())?;
                    iterations = value
                        .parse::<usize>()
                        .map_err(|_| "--iterations requires a whole number".to_owned())?;
                    iterations_seen = true;
                }
                "--windows-pipe" => {
                    if workload_seen {
                        return Err("only one workload may be selected".to_owned());
                    }
                    workload = Workload::WindowsPipe;
                    workload_seen = true;
                }
                "--renderer" => {
                    if workload_seen {
                        return Err("only one workload may be selected".to_owned());
                    }
                    workload = Workload::Renderer;
                    workload_seen = true;
                }
                "--help" | "-h" => return Err(help_text()),
                _ => return Err(format!("unrecognized argument: {argument}")),
            }
        }

        if !(MIN_ITERATIONS..=MAX_ITERATIONS).contains(&iterations) {
            return Err(format!(
                "--iterations must be between {MIN_ITERATIONS} and {MAX_ITERATIONS}"
            ));
        }
        Ok(Self {
            iterations,
            workload,
        })
    }
}

fn help_text() -> String {
    format!(
        "usage: anodrel-perf-lab [--iterations <{MIN_ITERATIONS}..{MAX_ITERATIONS}>] [--windows-pipe | --renderer]\n\
         Default workload: the owned in-process transport path, for 1 KiB and 64 KiB requests.\n\
         --windows-pipe measures the same requests across a real Windows named-pipe loopback.\n\
         --renderer measures the owned software rasterizer's drawing stages; it opens no window and performs no blit.\n\
         Default iterations: {DEFAULT_ITERATIONS}."
    )
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_ITERATIONS, Options, Workload};

    #[test]
    fn accepts_the_bounded_iteration_option() {
        assert_eq!(
            Options::parse(["--iterations".to_owned(), "10".to_owned()]),
            Ok(Options {
                iterations: 10,
                workload: Workload::InProcess
            })
        );
    }

    #[test]
    fn defaults_and_rejects_invalid_arguments() {
        assert_eq!(
            Options::parse(Vec::new()),
            Ok(Options {
                iterations: DEFAULT_ITERATIONS,
                workload: Workload::InProcess
            })
        );
        assert_eq!(
            Options::parse([
                "--windows-pipe".to_owned(),
                "--iterations".to_owned(),
                "10".to_owned()
            ]),
            Ok(Options {
                iterations: 10,
                workload: Workload::WindowsPipe
            })
        );
        assert!(Options::parse(["--iterations".to_owned(), "0".to_owned()]).is_err());
        assert!(
            Options::parse([
                "--iterations".to_owned(),
                "10".to_owned(),
                "--iterations".to_owned(),
                "20".to_owned()
            ])
            .is_err()
        );
        assert!(Options::parse(["--unknown".to_owned()]).is_err());
    }

    #[test]
    fn selects_the_renderer_workload_and_refuses_two_at_once() {
        assert_eq!(
            Options::parse(["--renderer".to_owned()]),
            Ok(Options {
                iterations: DEFAULT_ITERATIONS,
                workload: Workload::Renderer
            })
        );
        // Two workloads in one run would report both under a single benchmark
        // identifier, which is exactly the confusion the identifiers prevent.
        for pair in [
            ["--renderer", "--windows-pipe"],
            ["--windows-pipe", "--renderer"],
            ["--renderer", "--renderer"],
        ] {
            assert!(
                Options::parse(pair.map(str::to_owned)).is_err(),
                "{pair:?} was accepted"
            );
        }
    }
}
