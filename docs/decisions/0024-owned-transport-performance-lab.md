# Decision 0024: Measure owned transport performance with a first-party tool

**Status:** Accepted

**Date:** 2026-07-31

## Context

Anodrel aims to improve on Electron only where equivalent measurements support
that result. The native wire, authenticated transport, and core are owned code
whose cost can be measured without a third-party benchmark runtime. Timing
assertions in the test suite would be unreliable across developer machines, but
an undocumented ad-hoc benchmark would not produce comparable results.

## Decision

The native workspace provides `anodrel-perf-lab`, a first-party release tool.
Its default workload measures fixed 1,024-byte and 65,536-byte
`platform.ping` payloads through the owned frame codec, already-authenticated
transport session, and core host. Its optional `--windows-pipe` workload uses a
temporary current-session named-pipe loopback through those same layers. Both
use a fixed 200-request warmup, a bounded explicit iteration count, and
nearest-rank p50/p95/p99 results in nanoseconds. Their separate stable local
JSON report identifiers are documented in `docs/PERFORMANCE.md`.

The tool has no third-party runtime dependency. It deliberately does not claim
to measure cold start, application memory, frame performance, or Electron.
Each comparison requires its own equivalent workload, environment record, and
raw result.

## Consequences

- owned transport work has a repeatable release measurement command;
- timing does not become a brittle pass/fail unit-test condition;
- performance claims remain scoped to measured workloads; and
- startup, memory, rendering, and cross-runtime comparisons remain separately
  defined work.

## Revisit conditions

Revisit when an OS adapter, asynchronous scheduling layer, or public protocol
operation needs a new equivalent workload. A measurement tool must not become a
shipped runtime dependency.
