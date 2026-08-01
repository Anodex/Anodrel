//! Command-line options that keep a local benchmark bounded and repeatable.

pub const DEFAULT_ITERATIONS: usize = 5_000;
const MIN_ITERATIONS: usize = 10;
const MAX_ITERATIONS: usize = 100_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Options {
    pub iterations: usize,
}

impl Options {
    pub fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut iterations = DEFAULT_ITERATIONS;
        let mut iterations_seen = false;
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
                "--help" | "-h" => return Err(help_text()),
                _ => return Err(format!("unrecognized argument: {argument}")),
            }
        }

        if !(MIN_ITERATIONS..=MAX_ITERATIONS).contains(&iterations) {
            return Err(format!(
                "--iterations must be between {MIN_ITERATIONS} and {MAX_ITERATIONS}"
            ));
        }
        Ok(Self { iterations })
    }
}

fn help_text() -> String {
    format!(
        "usage: anodrel-perf-lab [--iterations <{MIN_ITERATIONS}..{MAX_ITERATIONS}>]\n\
         Measures the owned in-process transport path for 1 KiB and 64 KiB requests.\n\
         Default iterations: {DEFAULT_ITERATIONS}."
    )
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_ITERATIONS, Options};

    #[test]
    fn accepts_the_bounded_iteration_option() {
        assert_eq!(
            Options::parse(["--iterations".to_owned(), "10".to_owned()]),
            Ok(Options { iterations: 10 })
        );
    }

    #[test]
    fn defaults_and_rejects_invalid_arguments() {
        assert_eq!(
            Options::parse(Vec::new()),
            Ok(Options {
                iterations: DEFAULT_ITERATIONS
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
}
