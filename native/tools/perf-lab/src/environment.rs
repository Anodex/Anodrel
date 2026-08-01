//! Minimal, non-identifying local context for a benchmark report.

/// Stable machine context that helps compare local measurement runs without
/// collecting a computer name, user name, path, serial number, or network data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Environment {
    operating_system: &'static str,
    architecture: &'static str,
    logical_processors: Option<usize>,
}

impl Environment {
    pub(crate) fn from_parts(
        operating_system: &'static str,
        architecture: &'static str,
        logical_processors: Option<usize>,
    ) -> Self {
        Self {
            operating_system,
            architecture,
            logical_processors,
        }
    }

    pub fn current() -> Self {
        Self::from_parts(
            std::env::consts::OS,
            std::env::consts::ARCH,
            std::thread::available_parallelism()
                .ok()
                .map(std::num::NonZeroUsize::get),
        )
    }

    pub fn to_json(&self) -> String {
        let logical_processors = self
            .logical_processors
            .map_or_else(|| "null".to_owned(), |count| count.to_string());
        format!(
            "{{\"operatingSystem\":\"{}\",\"architecture\":\"{}\",\"logicalProcessors\":{logical_processors}}}",
            self.operating_system, self.architecture
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Environment;

    #[test]
    fn formats_only_non_identifying_measurement_context() {
        let environment = Environment::from_parts("windows", "x86_64", Some(16));

        assert_eq!(
            environment.to_json(),
            "{\"operatingSystem\":\"windows\",\"architecture\":\"x86_64\",\"logicalProcessors\":16}"
        );
    }
}
