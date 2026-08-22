# Anodrel Windows window lifecycle

**Status:** Windows foundation contract. The native host now supports multiple
native top-level windows on one UI thread. It is not a public application
window API yet.

## Purpose

Anodrel needs a direct equivalent of a basic desktop window lifecycle before a
future application can safely use multiple windows. The first host used one
process-global view slot and posted process quit when any window closed. That
was suitable for a smoke test but could not support a primary window and a
companion window safely.

## Contract

Each native window has an immutable, host-created view registered under its
Win32 handle before it is shown. Painting looks up that single view by handle;
content for one window cannot be drawn into another window merely because both
share a message loop.

The host creates all requested windows before it enters the standard User32
message loop. Closing one window removes only its own view. The UI thread exits
only when the final registered host window is destroyed. If creating a later
window fails, the host destroys every already-created window and returns a safe
failure rather than entering a partial message loop.

## Panic containment

The window procedure is `extern "system"`, which does not unwind, so a panic
escaping it becomes an immediate process abort — and an abort runs no
destructor. A defect while painting or servicing a timer would therefore strand
a verified product child with no host to shut it down, and leave a
notification-area entry on screen with nothing behind it.

Each window message is run inside a containment boundary. A panic ends the
message loop instead of the process: the loop returns, the host clears every
remaining view, and the ordinary drop paths shut down whatever those views
owned. The host exits with a failure status.

The panic payload is dropped rather than inspected. A message can carry
arbitrary values, so nothing derived from one reaches a protocol response, the
diagnostic ledger, or an application.

This is containment, not recovery. The host does not resume after a contained
panic, because the state that produced it is not known to be sound.

A removed view is dropped after the registry lock is released, never while it is
held. A session-window view first removes its exact host-private logical/native
mapping outside that lock. Removing a secondary then releases only that
secondary's portable resources. Removing the primary requests group-wide
shutdown, so each remaining group window closes through its own UI-thread timer
rather than leaving current primary-only bridges detached from a real surface.
The group, rather than its first window, retains a verified product session; it
ends the child and joins its two worker threads only after the final group view
leaves. Doing that under the process-wide registry lock would hold every other
window's message handling behind it, and would deadlock if a worker ever needed
to read the registry on its way out.

The existing package window and Startup Lab remain one-window host surfaces.
`--window-lab` is an Anodrel diagnostic that creates a primary and a companion
window to exercise the view registry and final-window shutdown behavior.

## Boundaries

All dimensions and views are created by the host, and no app protocol request
can create, enumerate, close, focus, or inject into a native window.

There are two narrow public exceptions. Protocol 1.14 lets an authenticated
session holding the `window.title` grant **propose** the title of the window it
already owns; the host validates the proposal and composes the displayed caption
with an application-name suffix the proposal cannot suppress or forge. Protocol
1.16 lets the separately granted `window.state` capability request only
minimise, maximise, or restore for that same session-owned window. Neither
request names a window, handle, target, geometry, focus action, or readback.
Protocol 1.21 separately permits `window.fullscreen.set` to choose only
reversible borderless fullscreen or windowed restoration for that same session
window; the host retains its native style and placement facts, and no monitor
or display control is exposed. Protocol 1.23 separately permits
`window.size.set` to choose only a bounded logical client area for that same
session window; the host derives its own framed rectangle and exposes no
position, monitor, DPI, bounds, or geometry readback. See
`docs/WINDOW_TITLE.md`, `docs/WINDOW_STATE.md`, `docs/WINDOW_FULLSCREEN.md`,
`docs/WINDOW_SIZE.md`, Decisions 0066, 0072, 0086, and 0088.
Everything else in this document is unchanged: the application still does not
learn that it has a window, where it is, or how large it is.

The Window Lab carries no package content, credentials, URLs, command data, or
privileged service. It is a native lifecycle test surface only. A public
window-management capability requires a versioned protocol, verified executable
identity, explicit permissions, cancellation rules, and its own threat-model
extension.

The host now also has an internal authenticated-session window entry point. It
accepts only one grouped set of mailboxes and native file resources created by
the registered-session adapter; titles and dimensions remain host-selected.
It does not expose application window creation, targets, enumeration, or
handles. `anodrel-windows-product-session` now provides the required
tracked-child and pipe shutdown owner. A provisioned signed application remains
required before this becomes an executable host path.

`docs/MULTI_WINDOW.md` and Decisions 0092 and 0093 define the future public
session-owned view model. That contract has no relationship to a raw registry
entry: its logical identifiers are session-scoped, bounded, and never native
handles. The host now implements its group lifetime, per-view resources, and
worker-to-UI creation handoff, but no application-facing part of the reserved
Protocol 1.25 surface is exposed until protocol, policy, SDK, mock-host, and
compatibility work ship together.

## Manual verification

From the repository root, run:

~~~text
cargo run --manifest-path native/Cargo.toml -p anodrel-windows-host -- --window-lab
~~~

Confirm that two **Anodrel Window Lab** windows open. Close either one: the
other must stay open. Close the remaining window: the host process must exit.

## Session-window group manual verification

The development-only Group Lab exercises the same dynamic creation handoff
that a future Protocol 1.25 request will use, without accepting any application
protocol command:

~~~text
cargo run --manifest-path native/Cargo.toml -p anodrel-windows-host -- --window-group-lab
~~~

Confirm that **Anodrel Window Group Lab** opens first and then a separately
captioned secondary window opens. Close the secondary: the primary remains.
Run it again and close the primary: the secondary closes shortly afterwards and
the process exits. This proves only host-owned group lifecycle and per-view
routing; it is not a public window API or a signed product-session test.
