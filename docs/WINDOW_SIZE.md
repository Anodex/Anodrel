# Anodrel Session Window Size

**Status:** Implemented for the direct Windows UI-session host; desktop
verification remains manual.

## Purpose

`window.size.set` lets an authenticated application choose the client-area
size of the one native window already associated with its own session. A useful
desktop surface must be able to make room for its content without turning that
need into a general window-management or desktop-inspection API.

The application supplies logical client dimensions, not a native outer-window
rectangle. On Windows the host converts them at the known window's current
DPI and add the host-selected non-client frame. This gives application layout a
stable 96-DPI unit while keeping title-bar, border, monitor, and process-DPI
details out of the protocol.

## Boundary

The request carries exactly a positive `width` and `height` in whole logical
pixels (1/96 inch), with these fixed bounds:

| Dimension | Inclusive bounds |
| --- | --- |
| `width` | 320 through 3840 |
| `height` | 240 through 2160 |

The dimensions describe the requested **client area**. They do not identify
the outer rectangle, native frame, monitor, DPI, position, current size, or
resulting size. The host preserves the window's current top-left position,
z-order, activation, and visibility. It does not restore, minimise, maximise,
focus, show, hide, move, create, close, enumerate, or target a window.

This operation is unavailable while Anodrel has applied its own reversible
fullscreen presentation for that session. The safe unavailable result does not
say whether fullscreen, a missing window, an expired bridge, or a native call
caused it. A size request while the window is normally minimised or maximised
may change only Windows' restored size; success remains no observation of
current or resulting presentation.

There is no size read, event, subscription, callback, aspect-ratio option,
minimum or maximum constraint, animation, position, monitor, display-mode,
native handle, or window target. Those are distinct authority or observation
boundaries and require separate decisions.

## Protocol

Protocol **1.23** reserves one exact operation:

| Field | Value |
| --- | --- |
| Operation | `window.size.set` |
| Payload | `{ "width": integer, "height": integer }` |
| Grant | `window.size` |
| Success | `{ "status": "applied" }` |
| Errors | `window.unavailable`, `window.busy` |

Both fields are required. Each is a base-10 JSON integer in the inclusive
range above; fractions, zero, negative values, strings, `null`, and unknown
fields are `request.payload_invalid`. A caller must not use a successful result
as evidence that the requested size is currently visible, that the window is
normal rather than minimised or maximised, or that any person saw it.

Installed application record version **1.12** adds `window.size` as a
strict superset of version 1.11. Older records that name this grant must be
invalid, preventing a host update from widening an existing application's
window authority.

## Native behavior and verification

The protocol worker places no more than one size request into its own
five-second session mailbox. The native window's UI thread alone converts
the bounded logical client dimensions at the known window's current DPI, ask
Windows for the matching framed outer rectangle, and resize without moving,
activating, or changing z-order. Windows failures and expired requests map
only to `window.unavailable`; a concurrent size request maps to
`window.busy`.

Portable, core, policy, and contract tests cover exact payload shape,
logical bounds, independent grant and record version, unavailable/busy mapping,
timeout clearing, and session isolation. The Windows adapter has pure
logical-to-physical conversion tests plus host tests for its per-view bridge.
Manual Windows verification must demonstrate the requested client area at
100% and non-100% display scaling, no position or activation change, and safe
refusal while Anodrel fullscreen is active.

## Deferred work

Position, bounds or DPI readback, monitor selection, resize constraints,
aspect-ratio enforcement, animation, geometry events, window creation,
cross-session targeting, exclusive display control, and non-Windows adapters
remain separate work.

See Decision 0088, `docs/WINDOW_FULLSCREEN.md`, and
`docs/WINDOW_LIFECYCLE.md`.
