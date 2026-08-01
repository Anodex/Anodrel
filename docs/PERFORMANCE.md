# Anodrel Performance Baseline

Anodrel will be considered better than Electron only where measurements support
the claim. Electron embeds Chromium and Node.js and uses a browser-style
multi-process model; it also advises developers to profile their specific apps,
avoid unnecessary modules, and keep work off the UI process. Anodrel will use
those concerns as testable design requirements, not as a claim that every
Anodrel application is automatically faster.

## Non-negotiable design properties

| Area | Anodrel requirement | Current protection |
| --- | --- | --- |
| Runtime ownership | No third-party shipped native runtime dependency. | Native dependency tree contains only Anodrel crates. |
| Native work | Keep raw OS calls isolated behind small adapters. | Win32 calls live only under `native/hosts/windows/src/win32/`; the renderer and brand crates reach no OS API at all. |
| Rendering | A frame must compose inside the animation timer's interval, or motion drops frames. | Measured and asserted by `an_animated_frame_fits_inside_the_timer_interval` in a release build; roughly 10 ms for the 1240×900 Startup Lab. |
| Message memory | Bound bytes before parsing and bound a single receive burst. | 64 KiB payload; four framed messages per receive. |
| Startup | Do no application I/O, network work, or deferred-service initialization before the first window is responsive. | Current host performs only its internal health check. |
| UI responsiveness | Never block the Windows message loop on stream I/O or expensive work. | Required by the transport contract; adapter work is not implemented yet. |
| Privilege | Native authority remains host-issued and operation-specific. | The transport frame carries no authority; core ignores supplied capability context. |

## Measurements before comparison claims

Before claiming an improvement over an Electron version of the same application,
record equivalent release builds on the same machine and workload:

1. cold-start time to first responsive window;
2. private bytes, working set, and idle CPU after 30 seconds;
3. 50th/95th/99th percentile request latency for 1 KiB and 64 KiB messages;
4. frame throughput and allocation behavior under fragmented and coalesced I/O;
5. UI responsiveness while a long-running native operation executes; and
6. installed package size, update size, and number of shipped runtime
   components.

Store hardware, OS build, workload, sample count, and raw results with every
comparison. A result is only meaningful for the named application and workload.

## Initial verification

`cargo tree --manifest-path native/Cargo.toml` verifies the native dependency graph.
Rendering is measured in a release build only; an unoptimised build is roughly
ten times slower and is not representative. Because software rendering costs
real time, expensive invariant layers are cached by the host rather than
recomposed each frame — see `docs/RENDERER.md`.
The wire and session unit tests verify framing, limits, fragmentation,
coalescing, authentication, and capability policy without timing-sensitive
assertions. The Windows adapter integration test exercises a real local named
pipe from connection through authenticated health response.

## Owned transport performance lab

`anodrel-perf-lab` is a first-party release measurement tool. It sends a
documented `platform.ping` request through Anodrel's frame codec,
already-authenticated `TransportSession`, and `CoreHost`. It measures the two
fixed wire payload sizes required above: **1,024 bytes** and **65,536 bytes**.

By default it isolates the work Anodrel owns in those three in-process layers.
`--windows-pipe` instead creates a temporary owner-restricted Windows named
pipe and measures the same authenticated request/response workload across that
pipe. Pipe creation, local connection, authentication, warmup, and close are
outside the timed samples. The two modes have distinct report identifiers and
must never be treated as interchangeable results.

Run it from the repository root in a release build:

~~~text
cargo run --release --manifest-path native/Cargo.toml -p anodrel-perf-lab -- --iterations 5000
cargo run --release --manifest-path native/Cargo.toml -p anodrel-perf-lab -- --windows-pipe --iterations 5000
~~~

`--iterations` accepts a whole number from 10 through 100,000 and defaults to
5,000. `--windows-pipe` selects the real Windows pipe loopback; omitting it
selects the in-process workload. The tool runs 200 unreported warmup requests
for each size. It writes one JSON object to standard output and performs no
file I/O. To retain a result, redirect standard output to an ignored local
`.anodrel/` directory along with the machine, OS build, power mode, compiler
version, and workload notes.

The report is a local tooling format, not a public protocol. Its v1 fields are:

| Field | Meaning |
| --- | --- |
| `benchmark` | Exact workload identifier: `anodrel.transport.in-process.v1` or `anodrel.transport.windows-pipe-loopback.v1`. |
| `iterations` | Measured requests per payload size, excluding warmup. |
| `measurements[].payloadBytes` | Exact encoded JSON payload size. |
| `measurements[].samples` | Number of reported latency samples. |
| `p50Nanoseconds`, `p95Nanoseconds`, `p99Nanoseconds` | Nearest-rank latency percentiles; rank is `ceil(percentile × samples / 100)`. |
| `meanNanoseconds` | Integer mean latency across reported samples. |
| `unit` | Always `nanoseconds`. |
| `scope` | Fixed statement of the layers being measured for the selected workload. |

This result must not be presented as startup time, process memory, rendering
performance, or an Electron comparison. The in-process workload must not be
presented as pipe latency. Both modes need an equivalent workload and recorded
environment before a cross-runtime comparison is published.

## Reference material

- Electron process model: <https://www.electronjs.org/docs/latest/tutorial/process-model>
- Electron security guidance: <https://www.electronjs.org/docs/latest/tutorial/security>
- Electron performance guidance: <https://www.electronjs.org/docs/latest/tutorial/performance>
