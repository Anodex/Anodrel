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
| Rendering | A frame must compose inside the animation timer's interval, or motion drops frames. | Measured and asserted by the `frame_budget` guards in a release build; see [Frame-cost guard](#frame-cost-guard). |
| Retained text | Moving a cached text run must not clone its antialiased coverage. | The Windows host stores one origin-zero mask and composites it through a bounded integer offset. |
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

## Static-window idle report

The direct Windows host has one fixed release diagnostic for its own idle CPU
and memory after a static native window is visible:

~~~text
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --idle-performance-report
~~~

Double-click `start-idle-performance-report.bat` from the repository root to
build and run the same fixed route.

For a release candidate, include the same measurement with the normal automated
evidence set:

~~~text
.\scripts\verify-windows-release.ps1 -IncludeIdleReport
~~~

This opt-in takes at least 30 seconds and opens the fixed diagnostic window, so
the default verifier remains free of desktop interaction for quick checks.

It shows one fixed host-rendered document, starts measuring after the first
paint, waits once for 30 seconds, prints one JSON record, and closes itself. Do
not interact with the window while it runs. Its `cpuTimeMicroseconds` is this
process's cumulative user-plus-kernel CPU time during the sample;
`cpuPercent` is the same value divided by the elapsed wall time without using a
floating-point estimate. `workingSetBytes` and `privateBytes` are the final
readings after that static-window sample.

The record's `scope` states that it excludes application sessions and process
trees. It has no pass/fail CPU limit: power mode, desktop activity, drivers, and
the scheduler all affect idle CPU. Retain the raw record with the operating
system build, hardware, power mode, and equivalent workload before comparing it
with another runtime. See Decision 0203.

## Frame-cost guard

The Startup Lab's reveal is driven by a 16 ms timer, so a frame that takes
longer than 16 ms to compose drops the one after it. Two release-only tests in
`native/hosts/windows/src/win32.rs` hold that line for the 1240×900 surface:

| Test | Asserts |
| --- | --- |
| `an_animated_frame_fits_inside_the_timer_interval` | The mean frame in the measured window fits the interval. |
| `no_single_frame_of_the_reveal_overruns_the_interval` | No individual frame in that window overruns it. |

Both report their measurement on success as well as on failure, so the number
can be read from a passing run with `-- --nocapture` and watched for drift.

### Why the statistic is a minimum, not a mean of one run

These are wall-clock measurements, so a single timed batch reports what the
machine did during that batch rather than what the renderer costs. The
difference is not small: **the same commit measured 8.5 ms per frame on an idle
desktop and 16–18 ms on a busy one**, which is how this guard came to fail
without anything in the renderer having changed. Checking the commit before the
session's work reproduced the same 16–18 ms, confirming it as an environment
effect rather than a regression.

Each guard therefore composes five batches and keeps the **cheapest**
observation of every frame. Contention can only make a batch slower, so the
cheapest observation is the closest one to the renderer's own cost, and a rise
in it is a real rise in that cost. On the reference machine below that turns a
run-to-run spread of several milliseconds into about 1%, and a 24-way CPU load
raises the reported figures by roughly 10% instead of doubling them.

Frames are kept apart rather than averaged into a single number because the
animation is not uniform: composing the mark's reveal costs several times what a
settled frame costs, and a mean hides which frame is nearest the interval.

Taking the cheapest observation also excludes one-time cache fills — the first
frame that builds the retained ambient layer costs about 13.6 ms and every later
frame at the same animation position costs about 0.9 ms. That is deliberate:
these guards measure sustained frame cost. The one-off fill is real, and is
recorded here rather than asserted, because asserting it would measure how many
times the batch has already run.

### Reference measurements

AMD Ryzen 9 7900X, Windows 11 Pro 10.0.26200, release build, idle machine:

| Figure | Before retaining the glow | Now |
| --- | --- | --- |
| Mean frame | 7.9 ms | **6.7 ms** of the 16 ms interval |
| Worst sustained frame | 10.0 ms | **8.0 ms**, around 760 ms into the reveal |
| First frame that fills the ambient layer | 13.6 ms, once | unchanged |

Where the time went in a reveal frame before that change, measured by timing
each stage of `startup_lab::draw`: the mark accounted for about 5.4 ms and the
status cards for about 3.0 ms, with the header, actions, footer, and the cached
backdrop together under 1 ms. Within the mark, the glow accounted for about
4.4 ms — 1.0 ms sampling the artwork's alpha into a coverage mask, 1.0 ms
blurring it, and 2.3 ms compositing it twice through a gradient. Retaining the
mask removed the first two; see
[Decision 0064](decisions/0064-retained-raster-effects-trade-bounded-fidelity.md).
The 2.3 ms composite is now the largest single cost in a frame.

Decision 0176 replaces only that diffuse glow's exact three-stop paint with a
512-sample bounded ramp. In one optimized `anodrel-perf-lab --renderer` run on
the reference machine (11 samples per stage), exact gradient mask fill averaged
**1.779 ms** and the equivalent quantized path averaged **1.113 ms** — 37% less
for that isolated operation. This is a same-process stage comparison, not a
claim that every frame became 37% faster: blending, rasterization, text, and
presentation still cost time. The exact and quantized stages remain together in
the workload so a later run can detect either regression.

## Host startup and memory report

~~~text
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --startup-report apps/sample/anodrel.application.json
~~~

Runs every check the host completes before a surface could open — package
verification, the core health check, the private pipe loopback, and the launch
preflight — then prints one JSON object and exits. It shares the exact sequence
the Startup Lab runs, so the reported time is the time a surface really waits
for rather than a second implementation that could drift from it.

It deliberately does not claim the single-instance mutex: a measurement must not
fight a running Startup Lab for it, or leave a claim that makes the next launch
think a surface is already open.

### What it measures, and what it excludes

| Field | Meaning |
| --- | --- |
| `startupMicroseconds` | Process start to startup checks complete. |
| `workingSetBytes` | Physical memory resident, including pages shared with other processes. |
| `privateBytes` | Committed memory this process cannot share. |
| `applicationId` | The validated package the checks ran against. |

**It stops before the window exists.** No window is created and nothing is
painted, so the time is a floor for cold start and must never be quoted as
time-to-first-frame, and the memory is the host's cost with nothing rendered.
The report's `scope` field says exactly this.

### Reference measurements

AMD Ryzen 9 7900X, Windows 11 Pro 10.0.26200, release build, sample package:

| Figure | Value |
| --- | --- |
| Startup checks, first run after build | ~10.1 ms |
| Startup checks, warm | ~1.4 ms |
| Working set | ~10.5 MB |
| Private bytes | ~1.8 MB |

The first run is several times the warm one, and that difference is the disk
reading the executable, not the host doing more work. Report both or say which.

### Before comparing this with another runtime

Two mistakes are easy here, and both would flatter Anodrel.

**A multi-process runtime needs its whole tree.** Electron runs a browser
process, a GPU process, a utility process, and one renderer per window.
Comparing one Anodrel process against one Electron process measures nothing.
Sum every process in the tree, and prefer **private bytes** — working set counts
pages shared between processes, so a tree double-counts them.

**The surfaces are not equivalent.** What Anodrel opens here is a diagnostic
surface built from four node kinds. An application with text input, lists,
images, or rich text would cost more than this, and that cost is not yet
measurable because the platform cannot express those surfaces. Until it can,
this figure is a floor for Anodrel and a full application for the other side.

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
5,000. `--windows-pipe` selects the real Windows pipe loopback and `--renderer`
selects the rasterizer workload below; omitting both selects the in-process
workload, and supplying two is refused rather than reported under one
identifier. The tool runs unreported warmup passes for each measurement. It
writes one JSON object to standard output and performs no file I/O. To retain a
result, redirect standard output to an ignored local `.anodrel/` directory along
with the machine, OS build, power mode, compiler version, and workload notes.

The report is a local tooling format, not a public protocol. Its v1 fields are:

| Field | Meaning |
| --- | --- |
| `benchmark` | Exact workload identifier: `anodrel.transport.in-process.v1`, `anodrel.transport.windows-pipe-loopback.v1`, or `anodrel.renderer.compose.v1`. |
| `iterations` | Measured passes per measurement, excluding warmup. |
| `measurements[].payloadBytes` | Exact encoded JSON payload size. Transport workloads only. |
| `measurements[].stage` | Drawing stage identifier. Renderer workload only. |
| `measurements[].pixels` | Pixels the stage composites over, so a cost can be read per pixel. Renderer workload only. |
| `measurements[].samples` | Number of reported latency samples. |
| `p50Nanoseconds`, `p95Nanoseconds`, `p99Nanoseconds` | Nearest-rank latency percentiles; rank is `ceil(percentile × samples / 100)`. |
| `meanNanoseconds` | Integer mean latency across reported samples. |
| `environment.operatingSystem` | Compile-target operating-system name reported by the Rust standard library. |
| `environment.architecture` | Compile-target architecture reported by the Rust standard library. |
| `environment.logicalProcessors` | Logical processors available to this process, or `null` when unavailable. It does not identify the computer or user. |
| `unit` | Always `nanoseconds`. |
| `scope` | Fixed statement of the layers being measured for the selected workload. |

The report deliberately omits computer/user names, paths, serial numbers,
network data, power state, OS build, and compiler version. Add the latter three
manually when retaining results, because they can materially affect comparison.

This result must not be presented as startup time, process memory, or an
Electron comparison. The in-process workload must not be presented as pipe
latency, and neither transport workload may be presented as rendering
performance. Each mode needs an equivalent workload and recorded environment
before a cross-runtime comparison is published.

## Renderer workload

~~~text
cargo run --release --manifest-path native/Cargo.toml -p anodrel-perf-lab -- --renderer --iterations 300
~~~

`--renderer` measures the owned software rasterizer one drawing stage at a
time. The [frame-cost guard](#frame-cost-guard) says whether a frame still fits
its interval; this says **which stage** to look at when it stops fitting.

It opens no window and performs no blit, so it is deliberately less than the
cost of a frame reaching the screen — the report's `scope` says so, and a
result from it must never be quoted as frame time.

The stages, their sizes fixed as constants so two runs are comparable:

| Stage | What it measures |
| --- | --- |
| `surface-clear` | Filling the whole 1240×900 surface with a flat colour. |
| `gradient-panel` | One rounded rectangle filled through a three-stop linear gradient, as a status card is. |
| `mask-blur` | Blurring a 366² coverage mask at the hero mark's radius, including the buffer copy a caller must make. |
| `mask-fill-gradient` | Compositing that blurred mask through a gradient. |
| `image-scale` | Compositing a bilinear-scaled 256² image into 220², as the mark's artwork is. |

The image is synthesized rather than taken from the brand crate. This tool
measures the rasterizer, and depending on the artwork would tie a performance
number to a design asset that is free to change. It is also a deliberate worst
case: nearly every pixel carries alpha, while the real mark has large fully
transparent regions that `draw_image` skips outright — so the real artwork
composites faster than this stage reports.

### Reference measurements

AMD Ryzen 9 7900X, Windows 11 Pro 10.0.26200, release build, 300 iterations,
median, expressed per pixel so the stages can be compared:

| Stage | p50 | Per pixel |
| --- | --- | --- |
| `surface-clear` | 72.7 µs | **0.07 ns** |
| `mask-blur` | 1.36 ms | 10 ns |
| `mask-fill-gradient` | 2.54 ms | 19 ns |
| `gradient-panel` | 1.39 ms | 33 ns |
| `image-scale` | 2.14 ms | 44 ns |

The spread is the finding. A flat fill costs essentially nothing per pixel; the
moment a paint has to be *evaluated* per pixel it costs two to three orders of
magnitude more. That is why `docs/RENDERER.md` names a quantised colour ramp as
the largest remaining renderer optimization, and this is the measurement that
would show whether it worked.

The p95 and p99 figures are much wider than the medians — `mask-blur` reaches
13.9 ms at p99 against a 1.36 ms median. Those tails are allocation and
scheduler noise, not rasterizer cost: each blur pass allocates a fresh
half-megabyte coverage buffer. Read the medians for the renderer and the tails
for the machine.

### Comparing two builds with this workload

Absolute numbers here drift with the machine by far more than a real
optimization is likely to be worth. A cold run measured `surface-clear` — which
touches no paint and cannot be affected by most renderer changes — 28% slower
than a warm one.

So a before-and-after must be **interleaved**, not sequential:

1. Build the unchanged binary and copy it aside.
2. Build the changed binary.
3. Alternate runs, several of each, and take the median per stage.
4. **Read the control stages first.** `surface-clear` and `mask-blur` use no
   paint; `image-scale` uses no gradient. If one of those moves as much as the
   stage under test, the run measured the machine and the result means nothing.

`docs/RENDERER.md` records one optimization this method rejected, and why the
rejection was the right outcome.

## Reference material

- Electron process model: <https://www.electronjs.org/docs/latest/tutorial/process-model>
- Electron security guidance: <https://www.electronjs.org/docs/latest/tutorial/security>
- Electron performance guidance: <https://www.electronjs.org/docs/latest/tutorial/performance>
