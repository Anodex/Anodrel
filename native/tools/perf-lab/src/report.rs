//! Stable JSON report formatting for local benchmark results.

use crate::arguments::Workload;
use crate::environment::Environment;

/// What one measurement varied, and how the report names it.
///
/// The transport workloads vary payload size; the renderer workload varies the
/// drawing stage. They are reported under separate benchmark identifiers with
/// separate field names rather than one widened field: `payloadBytes` is a
/// documented v1 field, and a result someone retained has to keep meaning what
/// it meant when it was written.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Dimension {
    /// Exact encoded request size, for a transport workload.
    PayloadBytes(usize),
    /// One named drawing stage and the pixels it composites over.
    Stage {
        /// Stable stage identifier, from the closed set in `workload.rs`.
        name: &'static str,
        /// Pixels the stage touches, so a cost can be read per pixel.
        pixels: u64,
    },
}

impl Dimension {
    fn to_json(self) -> String {
        match self {
            Self::PayloadBytes(bytes) => format!("\"payloadBytes\":{bytes}"),
            Self::Stage { name, pixels } => format!("\"stage\":\"{name}\",\"pixels\":{pixels}"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LatencyMeasurement {
    pub dimension: Dimension,
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
        Workload::Renderer => "anodrel.renderer.compose.v1",
    }
}

fn scope(workload: Workload) -> &'static str {
    match workload {
        Workload::InProcess => "owned wire, authenticated transport, and core only",
        Workload::WindowsPipe => {
            "owned Windows named pipe, wire, authenticated transport, and core"
        }
        Workload::Renderer => {
            "owned software rasterizer only; no window, presentation, or platform blit"
        }
    }
}

impl LatencyMeasurement {
    fn to_json(&self) -> String {
        format!(
            concat!(
                "{{{},\"samples\":{},",
                "\"p50Nanoseconds\":{},\"p95Nanoseconds\":{},",
                "\"p99Nanoseconds\":{},\"meanNanoseconds\":{}}}"
            ),
            self.dimension.to_json(),
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

    use super::{Dimension, LatencyMeasurement, Report};

    #[test]
    fn formats_a_machine_readable_report_without_dynamic_strings() {
        let report = Report {
            workload: Workload::InProcess,
            iterations: 10,
            environment: Environment::from_parts("windows", "x86_64", Some(16)),
            measurements: vec![LatencyMeasurement {
                dimension: Dimension::PayloadBytes(1_024),
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

    #[test]
    fn the_renderer_workload_reports_stages_rather_than_payload_sizes() {
        // A renderer result must not be mistakable for a transport one. It has
        // its own benchmark identifier, and its measurements name a stage
        // instead of carrying `payloadBytes`, which is a documented v1 field
        // that has to keep meaning what it meant.
        let report = Report {
            workload: Workload::Renderer,
            iterations: 10,
            measurements: vec![LatencyMeasurement {
                dimension: Dimension::Stage {
                    name: "mask-blur",
                    pixels: 133_956,
                },
                samples: 10,
                p50_nanoseconds: 10,
                p95_nanoseconds: 20,
                p99_nanoseconds: 30,
                mean_nanoseconds: 15,
            }],
            environment: Environment::from_parts("windows", "x86_64", Some(16)),
        };

        let json = report.to_json();
        assert!(json.contains("\"benchmark\":\"anodrel.renderer.compose.v1\""));
        assert!(json.contains("\"stage\":\"mask-blur\",\"pixels\":133956"));
        assert!(!json.contains("payloadBytes"));
        // The scope has to say what is missing, or a reader will take this for
        // the cost of a frame reaching the screen.
        assert!(json.contains("no window, presentation, or platform blit"));
    }
}
