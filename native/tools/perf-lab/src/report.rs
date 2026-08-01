//! Stable JSON report formatting for local benchmark results.

use crate::arguments::Workload;
use crate::environment::Environment;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LatencyMeasurement {
    pub payload_bytes: usize,
    pub samples: usize,
    pub p50_nanoseconds: u128,
    pub p95_nanoseconds: u128,
    pub p99_nanoseconds: u128,
    pub mean_nanoseconds: u128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Report {
    pub workload: Workload,
    pub iterations: usize,
    pub measurements: Vec<LatencyMeasurement>,
    pub environment: Environment,
}

impl Report {
    pub fn to_json(&self) -> String {
        let measurements = self
            .measurements
            .iter()
            .map(LatencyMeasurement::to_json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            concat!(
                "{{\"benchmark\":\"{}\",",
                "\"iterations\":{},\"measurements\":[{}],",
                "\"environment\":{},",
                "\"unit\":\"nanoseconds\",",
                "\"scope\":\"{}\"}}\n"
            ),
            benchmark_name(self.workload),
            self.iterations,
            measurements,
            self.environment.to_json(),
            scope(self.workload)
        )
    }
}

fn benchmark_name(workload: Workload) -> &'static str {
    match workload {
        Workload::InProcess => "anodrel.transport.in-process.v1",
        Workload::WindowsPipe => "anodrel.transport.windows-pipe-loopback.v1",
    }
}

fn scope(workload: Workload) -> &'static str {
    match workload {
        Workload::InProcess => "owned wire, authenticated transport, and core only",
        Workload::WindowsPipe => {
            "owned Windows named pipe, wire, authenticated transport, and core"
        }
    }
}

impl LatencyMeasurement {
    fn to_json(&self) -> String {
        format!(
            concat!(
                "{{\"payloadBytes\":{},\"samples\":{},",
                "\"p50Nanoseconds\":{},\"p95Nanoseconds\":{},",
                "\"p99Nanoseconds\":{},\"meanNanoseconds\":{}}}"
            ),
            self.payload_bytes,
            self.samples,
            self.p50_nanoseconds,
            self.p95_nanoseconds,
            self.p99_nanoseconds,
            self.mean_nanoseconds
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::{arguments::Workload, environment::Environment};

    use super::{LatencyMeasurement, Report};

    #[test]
    fn formats_a_machine_readable_report_without_dynamic_strings() {
        let report = Report {
            workload: Workload::InProcess,
            iterations: 10,
            environment: Environment::from_parts("windows", "x86_64", Some(16)),
            measurements: vec![LatencyMeasurement {
                payload_bytes: 1_024,
                samples: 10,
                p50_nanoseconds: 10,
                p95_nanoseconds: 20,
                p99_nanoseconds: 30,
                mean_nanoseconds: 15,
            }],
        };

        assert_eq!(
            report.to_json(),
            "{\"benchmark\":\"anodrel.transport.in-process.v1\",\"iterations\":10,\"measurements\":[{\"payloadBytes\":1024,\"samples\":10,\"p50Nanoseconds\":10,\"p95Nanoseconds\":20,\"p99Nanoseconds\":30,\"meanNanoseconds\":15}],\"environment\":{\"operatingSystem\":\"windows\",\"architecture\":\"x86_64\",\"logicalProcessors\":16},\"unit\":\"nanoseconds\",\"scope\":\"owned wire, authenticated transport, and core only\"}\n"
        );
    }

    #[test]
    fn identifies_the_windows_pipe_workload_separately() {
        let report = Report {
            workload: Workload::WindowsPipe,
            iterations: 10,
            measurements: Vec::new(),
            environment: Environment::from_parts("windows", "x86_64", Some(16)),
        };

        assert!(
            report
                .to_json()
                .contains("anodrel.transport.windows-pipe-loopback.v1")
        );
    }
}
