# Anodrel Windows native SDK

**Status:** Implemented and verified through every generated Windows native
template's isolated build and invited-pipe session test.

## Purpose

`anodrel-windows-ui-sdk` is the stable in-repository Windows entry point for a
compiled native Anodrel development application. It owns private invited-session
setup and exposes existing typed UI-session methods without exposing transport
or operating-system authority.

It is not a product package format, launcher, installer, identity system,
update client, cross-platform runtime, or registry-published package.

## Entry point

An application creates one session by calling
`WindowsUiSession::connect_from_stdin`. The SDK reads one `ANBI` invitation from
standard input, opens only its exact invited Windows pipe, authenticates before
application requests, and drops the invitation after authentication. It exposes
no constructor that accepts a pipe name, stream, token, capability list, or
native handle.

Connection errors are closed categories. They never include bootstrap bytes,
pipe names, tokens, raw Windows errors, raw host responses, or host diagnostics.

## Typed session surface

The facade preserves the documented typed operations from `anodrel-ui-client`:
strict v1/v2/v3 document replacement, bounded semantic-event reads,
whole-surface field snapshots, complete menu and tray replacement, opaque
secondary-view operations, one-way notification delivery, retained output
selection and text writing, and group close. It also exposes targetless
controls for the authenticated session's own host window:

| Method | Existing operation | Result |
| --- | --- | --- |
| `set_window_title` | `window.title.set` | accepted host-composed title proposal |
| `set_window_state` | `window.state.set` | accepted closed state request |
| `request_window_focus` | `window.focus.request` | accepted foreground request |
| `set_window_fullscreen` | `window.fullscreen.set` | accepted reversible presentation request |
| `set_window_size` | `window.size.set` | accepted bounded logical client-size request |
| `replace_tray_v1` | `tray.replace` | opaque semantic tray revision |
| `read_tray_actions` | `ui.events.read` | revision-bound tray action batch |
| `show_notification` | `notification.show` | accepted one-way host handover |
| `select_save_file_v2` | `dialog.save_file.v2` | cancelled or display path with opaque retained output reference |
| `write_selected_text` | `file.write_text` | accepted bounded one-use retained text write |

`WindowState`, `WindowFullscreenMode`, and `WindowSize` are re-exported by the
facade, so an application does not import an implementation crate. The methods
have no window identifier, native handle, geometry readback, presentation
readback, or cross-window route; the host remains authoritative for the
separate grant attached to each operation. Every method uses its minimum
documented protocol version internally; applications cannot choose an arbitrary
operation or protocol version.

Whether a method succeeds still depends on the host-issued grant. The SDK does
not declare, request, inspect, or broaden capabilities.

## Compatibility

The first surface is version `0.1.0` inside this repository. Additive changes
need public documentation and generated-template compatibility coverage. A
removal or incompatible type change requires a new decision and a new `0.2`
minor line. Registry publication is intentionally separate work.

The generated UI, menu, context-menu, tray, notification, file-write, form,
live-status, multi-window, scroll-window, and window-controls projects are the
real consumers. Their isolated release builds and authenticated Windows-pipe
sessions prove that the SDK has no hidden host-source dependency. The
window-controls project specifically covers every targetless window-control
method under the host's exact fixed grants.

See Decision 0104 and `docs/NATIVE_CLIENT.md` for the lower-level private
transport contract.
