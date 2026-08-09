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

The existing package window and Startup Lab remain one-window host surfaces.
`--window-lab` is an Anodrel diagnostic that creates a primary and a companion
window to exercise the view registry and final-window shutdown behavior.

## Boundaries

All titles, dimensions, and views are created by the host. The current text
package still controls none of these values, and no app protocol request can
create, enumerate, close, focus, or inject into a native window.

The Window Lab carries no package content, credentials, URLs, command data, or
privileged service. It is a native lifecycle test surface only. A public
window-management capability requires a versioned protocol, verified executable
identity, explicit permissions, cancellation rules, and its own threat-model
extension.

The host now also has an internal authenticated-session window entry point. It
accepts only one grouped set of mailboxes and native file resources created by
the registered-session adapter; titles and dimensions remain host-selected.
It does not expose application window creation, targets, enumeration, or
handles. A provisioned signed application and tracked-child shutdown policy are
still required before this becomes a product launch path.

## Manual verification

From the repository root, run:

~~~text
cargo run --manifest-path native/Cargo.toml -p anodrel-windows-host -- --window-lab
~~~

Confirm that two **Anodrel Window Lab** windows open. Close either one: the
other must stay open. Close the remaining window: the host process must exit.
