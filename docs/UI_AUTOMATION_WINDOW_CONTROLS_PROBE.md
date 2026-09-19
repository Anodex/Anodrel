# Anodrel UI Automation window-controls probe

## Purpose

`--uia-window-controls-probe <native-client.exe>` is a fixed Windows
development diagnostic. It starts one compiled child with the existing narrow
window-control grants, then a host-owned MTA UI Automation client invokes its
eight fixed semantic actions in order.

The child receives only existing acknowledgement responses. The host keeps the
window handle, UI Automation elements, screen rectangles, and observations
private. No application can select an automation target or learn title, size,
state, monitor, foreground, or probe results. The suite supplies the compiled
first-party child; this development route does not establish a code-identity
claim for an arbitrary executable path.

## Verified sequence

The probe requires these host-visible results from its one temporary session:

1. the fixed title proposal changes the UI Automation window name to the
   host-composed caption;
2. the fixed logical client-size request changes the visible window rectangle;
3. maximise changes that rectangle and restore returns it to the resized one;
4. the foreground request completes through its existing acknowledgement path;
5. fullscreen changes the rectangle and windowed mode returns it to the same
   resized rectangle; and
6. the final action reaches the ordinary semantic-event and session-close path.

It does not prove that Windows foregrounded the window, that a person saw any
transition, minimise behaviour, client dimensions at every display scale,
position/z-order invariants, or multi-monitor fullscreen selection. Those
remain documented manual Windows checks.

## Running it

Run the complete eight-probe suite from the repository root:

~~~powershell
.\scripts\verify-windows-accessibility.ps1
~~~

The suite builds the fixed first-party child, opens only temporary host-owned
windows, and creates no trust, package, installer, network, or persistent user
state. On 2026-09-19, this probe and the other seven direct UI Automation
probes passed against the release build.

## Boundary

This is a host-local verification route using direct Windows UI Automation APIs
and first-party Anodrel code. Its action IDs, title proposal, size, and
sequence are compiled constants. It accepts no document, selector, coordinate,
title, window, monitor, result, callback, or protocol input; the supplied
development executable provides its fixed document through the existing
session route.

It complements, but cannot replace, Narrator, Inspect, and visible native
acceptance. See Decisions 0069 and 0226.
