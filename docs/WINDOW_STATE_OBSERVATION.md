# Anodrel Window State Observation

**Status:** Implemented on Windows for Protocol 1.30. Manual verification of
the signed product fixture remains open because it requires local machine trust.

## Purpose

`window.state.get` supplies one current presentation snapshot for the native
window that the authenticated session already owns. It exists so an application
can initialise an owned title bar correctly after startup or refresh it after
its own presentation request. It is not a general native-window API.

The existing Protocol 1.16 `window.state.set` remains a separate write-only
capability. Its success response stays `{ "status": "applied" }` and never
contains a resulting state.

## Protocol contract

| Field | Value |
| --- | --- |
| Protocol | 1.30 or later |
| Operation | `window.state.get` |
| Payload | `{}` exactly |
| Grant | `window.state.read` |
| Success | `{ "state": "minimized" \| "maximized" \| "restored" }` |
| Errors | `window.unavailable`, `window.busy` |

The request has no selector or option. Its response contains no native handle,
window ID, position, bounds, display, monitor, DPI, visibility, z-order,
focus, fullscreen value, timestamp, event sequence, or platform detail.

The host finds the window from the authenticated session. A request cannot be
directed at another application, session, secondary view, host diagnostic, or
operating-system surface.

## Snapshot semantics

The response is a point-in-time answer from the host window's owning UI thread.
The state can change immediately afterward through a person using Windows, a
host lifecycle transition, or the application's later separate state request.
There is no `onChanged`, subscription, callback, background receiver, event
history, or delivery promise. Applications must not treat one result as a
long-lived lease over window presentation.

## Capability and compatibility

`window.state.read` is separate from `window.state`: a machine policy can allow
an application to request a presentation state without letting it read one, or
allow a title-bar refresh without granting presentation control. Installed
record version 1.17 is the first version allowed to name the read grant and is
a strict superset of version 1.16. Older records naming it are invalid.

Hosts that lack an associated UI surface return `window.unavailable`. At most
one state-read request may wait for one session UI thread at a time. A second
request returns `window.busy`; a five-second timeout clears its pending entry
and returns `window.unavailable` so a stuck UI thread cannot strand a session.

## Anodex migration use

Anodex's existing Electron title bar reads its own maximised state when it
mounts. `@anodrel/anodex-adapter` now uses this operation for an initial
snapshot and after its own request, while the Electron adapter remains in use
during migration. This specification does not add a React or browser renderer
to Anodrel and does not claim Anodex can run on Anodrel today. See
`docs/ANODEX_ADAPTER.md` and Decision 0117.

The separately implemented `window.state.changes.read` companion covers one
coalesced later state only; it does not change this snapshot operation or add a
subscription. See `docs/WINDOW_STATE_CHANGES.md` and Decision 0118.

## Verification

- Protocol, SDK, mock-host, and core tests cover the exact empty payload,
  independent grant/version gate, closed response, unavailable/busy mapping,
  and no leaked observation fields;
- mailbox tests cover take-once completion, timeout cleanup, and isolation;
- Windows-host tests cover session-owned UI-thread routing and exhaustive
  `IsIconic`/`IsZoomed` reduction without exposing a native handle; and
- the `@anodrel/anodex-adapter` contract test proves an Anodex-shaped title bar
  reads its initial label and refreshes after its own request without Electron.

The direct development diagnostic completes the Windows native check without
changing machine trust:

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --sample-ui-window-state-read-client $nodePath $clientPath
~~~

It verifies the newly created window starts restored, then observes maximized
and restored after its own requests. The separately provisioned signed fixture
does not yet declare this version-1.17 grant, so it is not presented as product
fixture evidence for this new capability.
