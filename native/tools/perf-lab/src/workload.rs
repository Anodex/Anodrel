//! Fixed transport workload construction and percentile calculation.

use std::{hint::black_box, time::Instant};

use anodrel_core::HostPolicy;
use anodrel_transport::{SessionCredentials, TransportSession, authentication_message};
use anodrel_windows_pipe::measure_loopback_request;
use anodrel_wire::encode_json;

use crate::arguments::Workload;
use crate::environment::Environment;
use crate::report::{Dimension, LatencyMeasurement, Report};

mod renderer;

const WARMUP_ITERATIONS: usize = 200;
const PAYLOAD_SIZES: [usize; 2] = [1_024, 64 * 1_024];
const SESSION_ID: &str = "anodrel-performance-session";
const TOKEN: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const REQUEST_PREFIX: &str = concat!(
    "{\"protocolVersion\":{\"major\":1,\"minor\":0},",
    "\"kind\":\"request\",\"requestId\":\"latency\",",
    "\"operation\":\"platform.ping\",\"payload\":{\"sentAt\":\""
);
const REQUEST_SUFFIX: &str = "\"}}";

pub fn measure(workload: Workload, iterations: usize) -> Result<Report, String> {
    if iterations == 0 {
        return Err("performance measurements require at least one iteration".to_owned());
    }
    let measurements = match workload {
        Workload::Renderer => renderer::measurements(iterations)?,
        Workload::InProcess | Workload::WindowsPipe => PAYLOAD_SIZES
            .into_iter()
            .map(|payload_bytes| measure_payload(workload, payload_bytes, iterations))
            .collect::<Result<Vec<_>, _>>()?,
    };
    Ok(Report {
        workload,
        iterations,
        measurements,
        environment: Environment::current(),
    })
}

fn measure_payload(
    workload: Workload,
    payload_bytes: usize,
    iterations: usize,
) -> Result<LatencyMeasurement, String> {
    let request = request_json(payload_bytes);
    let samples = match workload {
        Workload::InProcess => in_process_samples(&request, iterations)?,
        Workload::WindowsPipe => windows_pipe_samples(&request, iterations)?,
        // Unreachable: `measure` routes the renderer workload before it can
        // reach a per-payload measurement, which is the point of the split.
        Workload::Renderer => {
            return Err("the renderer workload does not vary payload size".to_owned());
        }
    };
    latency_measurement(payload_bytes, samples)
}

fn in_process_samples(request: &str, iterations: usize) -> Result<Vec<u128>, String> {
    let frame = encode_json(request).map_err(|error| error.to_string())?;
    let mut session = authenticated_session()?;

    for _ in 0..WARMUP_ITERATIONS {
        receive(&mut session, &frame)?;
    }

    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let started = Instant::now();
        receive(&mut session, &frame)?;
        samples.push(started.elapsed().as_nanos());
    }
    Ok(samples)
}

fn windows_pipe_samples(request: &str, iterations: usize) -> Result<Vec<u128>, String> {
    let policy = HostPolicy::new("org.anodrel.performance", Vec::new(), "anodrel-perf-lab")
        .map_err(str::to_owned)?;
    measure_loopback_request(policy, request, WARMUP_ITERATIONS, iterations)
        .map(|measurements| {
            measurements
                .into_iter()
                .map(|sample| sample.as_nanos())
                .collect()
        })
        .map_err(|_| "Windows named-pipe loopback measurement did not complete".to_owned())
}

fn latency_measurement(
    payload_bytes: usize,
    samples: Vec<u128>,
) -> Result<LatencyMeasurement, String> {
    measurement(Dimension::PayloadBytes(payload_bytes), samples)
}

fn measurement(dimension: Dimension, mut samples: Vec<u128>) -> Result<LatencyMeasurement, String> {
    if samples.is_empty() {
        return Err("performance measurement did not produce samples".to_owned());
    }
    samples.sort_unstable();
    let mean_nanoseconds = samples.iter().sum::<u128>() / samples.len() as u128;
    Ok(LatencyMeasurement {
        dimension,
        samples: samples.len(),
        p50_nanoseconds: percentile(&samples, 50),
        p95_nanoseconds: percentile(&samples, 95),
        p99_nanoseconds: percentile(&samples, 99),
        mean_nanoseconds,
    })
}

fn authenticated_session() -> Result<TransportSession, String> {
    let policy = HostPolicy::new("org.anodrel.performance", Vec::new(), "anodrel-perf-lab")
        .map_err(str::to_owned)?;
    let mut session = TransportSession::new(
        policy,
        SessionCredentials::new(SESSION_ID, TOKEN).map_err(|error| error.to_string())?,
    );
    let authentication =
        authentication_message(SESSION_ID, TOKEN).map_err(|error| error.to_string())?;
    receive(
        &mut session,
        &encode_json(&authentication).map_err(|error| error.to_string())?,
    )?;
    Ok(session)
}

fn receive(session: &mut TransportSession, frame: &[u8]) -> Result<(), String> {
    black_box(
        session
            .receive(black_box(frame))
            .map_err(|error| error.to_string())?,
    );
    Ok(())
}

fn request_json(payload_bytes: usize) -> String {
    assert!(payload_bytes >= REQUEST_PREFIX.len() + REQUEST_SUFFIX.len());
    let timestamp_bytes = payload_bytes - REQUEST_PREFIX.len() - REQUEST_SUFFIX.len();
    format!(
        "{REQUEST_PREFIX}{}{REQUEST_SUFFIX}",
        "x".repeat(timestamp_bytes)
    )
}

fn percentile(samples: &[u128], percentage: usize) -> u128 {
    assert!(!samples.is_empty());
    assert!(percentage <= 100);
    let rank = (percentage * samples.len()).div_ceil(100);
    samples[rank.saturating_sub(1)]
}

#[cfg(test)]
mod tests {
    use anodrel_wire::MAX_PAYLOAD_BYTES;

    use crate::arguments::Workload;

    use super::{measure, percentile, request_json};

    #[test]
    fn builds_exact_wire_payload_sizes() {
        assert_eq!(request_json(1_024).len(), 1_024);
        assert_eq!(request_json(MAX_PAYLOAD_BYTES).len(), MAX_PAYLOAD_BYTES);
    }

    #[test]
    fn uses_nearest_rank_percentiles() {
        let samples = [10, 20, 30, 40];
        assert_eq!(percentile(&samples, 50), 20);
        assert_eq!(percentile(&samples, 95), 40);
        assert_eq!(percentile(&samples, 99), 40);
    }

    #[test]
    fn runs_the_owned_transport_workload() {
        let report = measure(Workload::InProcess, 10).expect("transport workload succeeds");
        assert_eq!(report.measurements.len(), 2);
        assert!(
            report
                .measurements
                .iter()
                .all(|measurement| measurement.samples == 10)
        );
    }

    #[test]
    fn runs_the_windows_pipe_workload() {
        let report = measure(Workload::WindowsPipe, 10).expect("pipe workload succeeds");
        assert_eq!(report.workload, Workload::WindowsPipe);
        assert_eq!(report.measurements.len(), 2);
        assert!(
            report
                .measurements
                .iter()
                .all(|measurement| measurement.samples == 10)
        );
    }

    #[test]
    fn rejects_an_empty_measurement() {
        assert!(measure(Workload::InProcess, 0).is_err());
    }
}
