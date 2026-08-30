# Decision 0131: Linux development-window Lab owns one session and view

- Status: Accepted
- Date: 2026-08-30

## Context

Decision 0130 gives a Linux host one explicit owner for an invited development
child and authenticated worker. Decisions 0128 and 0129 separately prove that
a fixed Anodrel canvas can reach a Wayland compositor and receive one local
diagnostic activation. Until those lifetimes meet, a child can end while a
diagnostic view remains open, or a desktop close can leave a child alive until
an unrelated caller notices it.

The portable close signal is intentionally only a coalescing host-local flag.
It carries no Linux descriptor, callback, process status, timing, or
application-visible event. Making it a generic event-loop primitive would
couple the portable core to one operating system's waiting mechanism.

## Decision

Add `anodrel-linux-development-window`, a development-only composition adapter
that owns exactly one `RunningLinuxDevelopmentSession` and one fixed
`LinuxWaylandLab` together. It accepts only the existing host-selected policy,
opaque session ID, and exact `LinuxBootstrapProgram`; it accepts no document,
title, application event handler, compositor object, native handle, or child
argument.

The composition starts the private child session before opening the fixed
Wayland Lab. If opening the view fails, normal session destruction stops and
joins the child and worker. While the view is live, the adapter waits for the
direct Wayland stream for at most 50 ms at a time after already-buffered input
has been exhausted. A kernel `poll` wait wakes immediately for compositor
input; a timeout merely lets the host inspect its already-existing close flag.
No timer, frame callback, thread, allocation, or application-visible poll
route is added. Interrupted waits recompute the remaining deadline so the
fixed interval does not stretch indefinitely.

Either the child or authenticated worker ending sets the private close flag.
The next bounded window wait drops the view and then finishes the existing
session. A compositor close follows the same finish path. The resulting
desktop outcome is one closed development-Lab event; it does not reveal which
side ended first, a child status, a process identifier, an endpoint, or an
operating-system failure.

## Consequences

- The first Linux child and visible first-party view now share one host-owned
  lifetime without adding a Linux application or product host.
- A live session has no busy wait: inactive iterations block in the kernel for
  at most 50 ms and retain no new state.
- The direct Wayland adapter remains responsible for compositor wire handling;
  the lifecycle adapter remains responsible for child shutdown and joining.
- The composed diagnostic can be verified without a compositor through its
  pure close-priority tests, and manually on a Wayland desktop with the fixed
  first-party held-session client.

## Deliberately absent

- application documents, IPC delivery, package loading, executable identity,
  product policy, installation, updates, or a public Linux host SDK;
- public pointer, keyboard, focus, accessibility, geometry, title, window,
  child-lifecycle, callback, timer, or native-event surfaces;
- a toolkit, libwayland, X11/XWayland fallback, graphics runtime, or a generic
  cross-platform descriptor/event-loop abstraction.

## Alternatives considered

**Add a Linux event handle to `SessionCloseSignal`.** That would make a
portable lifecycle type own one operating system's wake primitive. Refused.

**Spin or sleep in application code while checking session state.** That would
make shutdown timing, CPU use, and native authority depend on an application.
Refused.

**Add a background waiter or a GUI toolkit.** Both introduce a second lifetime
and either a hidden callback path or an unowned shipped runtime. Refused.

## Revisit conditions

Revisit before application document composition, a public Linux window API,
input delivery, accessibility, multi-window ownership, product identity,
installed policy, packaging, updates, or a different host-close wake design.
