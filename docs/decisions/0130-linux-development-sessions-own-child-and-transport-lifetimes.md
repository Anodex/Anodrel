# Decision 0130: Linux development sessions own child and transport lifetimes

- Status: Accepted
- Date: 2026-08-30

## Context

Linux now has separately verified components for an authenticated abstract
Unix-socket server, ANLI child invitation, and exact `execve` development
launcher. Each deliberately stops before deciding how a host owns the other
components. Starting them independently risks an invited child surviving a
closed host or a worker waiting after its child has ended.

The Windows verified-product coordinator has the same ownership problem, but
it depends on Windows policy and UI resources. Reusing it would create a false
cross-platform product claim. Linux has no package identity, installed policy,
application window, or product launcher yet.

## Decision

Add `anodrel-linux-development-session`, a Linux-only host adapter that joins
one host-created `LinuxPipeServer`, one converted ANLI invitation, one
host-selected `LinuxBootstrapProgram`, one launched child, a pipe worker, and
a child-exit watcher.

The caller supplies an already validated host policy, opaque session ID, and
already validated `LinuxBootstrapProgram`; it cannot supply an endpoint,
token, command line, environment, output route, native handle, process ID, or
signal selection. The coordinator adds no protocol operation, capability,
application document, window API, or service.

The pipe worker and exit watcher run off any future Linux UI thread. Either
ending requests the same host-owned close signal, stops pending pipe work, and
asks the tracked child to end. Explicit `finish` and `Drop` request that same
shutdown before joining both workers. A child gets one short fixed SIGTERM
grace period during host shutdown; a still-running tracked child then receives
the adapter's fixed SIGKILL fallback before the exit watcher can finish. No
application chooses either signal.

This is a development-session lifecycle foundation only. It has no Wayland
composition yet: a future Linux window host must retain one running session
while it owns its native view and consume the same close signal without adding
a raw callback.

## Consequences

- Linux obtains one explicit owner for a private child and its authenticated
  transport instead of relying on call-stack timing or detached worker threads.
- Shutdown has bounded polling and no user-paced join: it first stops the
  listener and child, then joins workers that are already returning.
- Existing Linux child and transport tests remain independent; new lifecycle
  tests can prove launch, worker ownership, clean shutdown, and safe failure
  categories with only first-party code and Linux APIs.
- The close signal is host-local and coalescing. It conveys no exit code,
  process state, endpoint, native error, application callback, or protocol
  event.

## Deliberately absent

- executable identity or validation, machine policy, package loading,
  installed application records, restart, background mode, output capture,
  user-selected command data, or a public process API;
- Wayland window creation, UI document composition, pointer delivery,
  accessibility, menus, storage services, package installation, updates, or
  macOS support;
- child exit-code, PID, handle, signal, or timing readback to applications.

## Alternatives considered

**Keep the components separate.** This leaves shutdown correctness dependent
on whichever caller starts them. Refused.

**Reuse the Windows product coordinator.** Its policy, native UI group, and
Windows-only primitives do not exist on Linux. Refused.

**Expose child lifecycle through the protocol.** That would make native process
state application authority before Linux has an application host. Refused.

## Revisit conditions

Revisit before adding a Linux verified-product session, native window
composition, package identity, application-selected lifetime behavior, restart,
background work, multiple children, or a public lifecycle operation.
