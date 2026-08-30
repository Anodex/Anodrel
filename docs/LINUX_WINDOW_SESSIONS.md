# Linux development window sessions

## Status

`anodrel-linux-development-window` is a development-only Linux host-composition
adapter. It holds one existing private child/transport session and one fixed
Wayland Lab view under the same host lifetime. It is not a Linux application
host, product session, package loader, or public SDK.

## Ownership

~~~text
host-selected policy + session ID + exact first-party child
    │
    ├── one RunningLinuxDevelopmentSession
    │     ├── authenticated abstract-socket worker
    │     └── watched invited child
    │
    └── one LinuxWaylandLab fixed diagnostic view
              │
              ▼
          bounded host event loop
~~~

The composition begins the existing child session and then opens the fixed
Wayland view. It preserves the launch adapter's no-argument child rule and
does not accept application data, a document, a title, a compositor object,
native handle, input callback, or child lifecycle choice.

## Closing rules

The development-session adapter's coalescing close signal remains private to
the host. After handling already-buffered Wayland input, the fixed view waits
in the kernel for compositor input for at most 50 ms. A Wayland message wakes
it immediately; an idle timeout checks only that private signal. This keeps
idle CPU work at zero while bounding an external child/worker end to the next
host close check.

If the child or authenticated worker ends, the host first sends the fixed
best-effort Wayland teardown sequence, drops the view connection, and then
finishes the session. If the desktop asks the fixed view to close, the host
follows the same session-finish path. Neither path exposes the side that ended
first, a PID, exit code, signal, endpoint, token, native error, event, or
callback to an application.

## Run and verify manually

On a local little-endian Linux Wayland desktop, run:

~~~text
scripts/start-linux-session-window-lab.sh
~~~

Expected result:

1. The fixed **Anodrel Linux Lab** appears and remains open while its
   compiled first-party development child is alive.
2. Clicking the fixed lower panel once shows its completed local diagnostic
   appearance when the desktop provides a pointer.
3. Closing the desktop view ends the held child and the process exits normally.

The automated suite proves close-signal priority, fixed wait bounds, child
lifecycle ownership, Wayland wire rules, and the fixed canvas. It does not
start a compositor. A green automated run therefore does not prove desktop
decoration, physical pointer use, compositor compatibility, or visual closure.

## Deliberate limits

This is only the first shared Linux child/view lifetime. It does not load an
application document, route authenticated application UI, expose input,
provide a product identity, validate an executable, install a package, update
software, or make a Linux product claim.

See Decisions 0128 through 0132, docs/LINUX_SESSIONS.md, and
docs/LINUX_WINDOWING.md.
