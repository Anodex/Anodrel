# Decision 0203: Windows idle performance report measures one static window

**Status:** Accepted

**Date:** 2026-09-05

## Context

The startup report stops before window creation, while the release frame guard
measures an animated composition batch. Neither answers how much CPU and memory
the actual host process uses after a normal static native surface is shown and
left alone. That measurement is required before Anodrel can make a fair
same-workload comparison with another desktop runtime.

A polling loop, an application-selected workload, a process-tree reader, or a
universal CPU threshold would turn a measurement into either an added idle cost
or an unreliable claim about another machine.

## Decision

The Windows host provides one development-only `--idle-performance-report`
route. It opens one fixed directly rendered document window, starts sampling
only after its first paint completes, waits once for 30 seconds, then samples
only the current process through direct Kernel32 and Psapi calls before closing
the window.

The report uses the stable local benchmark name `anodrel.host.idle.v1` and
contains sample duration, cumulative CPU time during that duration, a
fixed-point CPU percentage, final working set, final private bytes, and a scope
statement. It takes no application, document, window, timing, process, file,
network, installer, or policy input. It neither exposes a protocol operation nor
creates a background monitor.

The command reports a measurement but has no CPU-performance pass/fail
threshold. A static window's CPU use changes with the desktop, power state,
drivers, and operating-system scheduling; equivalent builds must be compared
on the same named environment.

## Consequences

- the release evidence can include a real idle host window rather than only a
  no-window startup floor;
- the measurement adds exactly one delayed UI-thread wakeup and no idle polling;
- application authority and product behaviour remain unchanged; and
- any cross-runtime statement still requires an equivalent application workload
  and recorded environment.

## Revisit conditions

Revisit when a released application establishes a concrete idle workload with a
defined process tree, background policy, or resource target. Those requirements
need their own bounded contract rather than widening this fixed host diagnostic.
