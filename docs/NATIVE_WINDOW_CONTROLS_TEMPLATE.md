# Anodrel development native window-controls template

**Status:** The fixed contract is accepted. The generator, Windows host route,
and compatibility checks are implemented with the accompanying Decision 0105
change; the visible Windows walkthrough remains a manual acceptance check. This
is a Windows development template, not a product packaging or application-
identity format.

## Purpose

The window-controls template creates a small first-party Rust executable that
visibly walks through Anodrel's existing targetless controls for its one
authenticated host window. A person advances one fixed document at a time: the
application proposes a title, requests a logical client size, maximises,
restores, asks Windows for attention, enters fullscreen, returns to windowed
presentation, and closes its own session.

It does not make the executable trusted and does not turn these controls into a
general window-management API. The generated application never receives a
window handle, identity, bounds, title, display, focus state, fullscreen state,
event, callback, or delivery confirmation. The host still owns every native
operation and may safely refuse one.

## Generator contract

`anodrel-native-app-tool init-window-controls <destination> <project-slug>
<display-label>` accepts the same validated destination, Cargo-compatible
project slug, and bounded display label as the existing generators. It refuses
an existing destination and writes only:

~~~text
my-native-window-controls-app/
|- Cargo.toml
|- README.md
`- src/
   `- main.rs
~~~

Every Anodrel dependency path is relative to the local checkout. The generator
does not install, run, sign, package, register, trust, or assign identity to
the executable. It accepts no capability list, native handle, window ID, title,
size, position, monitor, state, focus option, document path, URL, or
configuration value.

## Typed client contract

The generated source imports only `anodrel-windows-ui-sdk` and uses these
closed existing methods:

| Method | Fixed input | Protocol | Required grant |
| --- | --- | --- | --- |
| `replace_document_v1` | one fixed strict v1 document per walkthrough step | 1.3 | `ui.document.write` |
| `read_actions` | none | 1.3 | `ui.events.read` |
| `set_window_title` | one fixed valid title proposal | 1.14 | `window.title` |
| `set_window_state` | `maximized`, then `restored` | 1.16 | `window.state` |
| `request_window_focus` | none | 1.20 | `window.focus` |
| `set_window_fullscreen` | `fullscreen`, then `windowed` | 1.21 | `window.fullscreen` |
| `set_window_size` | one fixed bounded `960 × 640` logical client size | 1.23 | `window.size` |
| `close` | none | existing | `session.close` |

Success from every window method means only that the host accepted and issued
the associated request. It never means the requested title, state, focus,
fullscreen, or client size is currently observable. In particular, Windows is
free to decline a foreground request under its normal foreground policy.

## Development host session

The implemented command is:

~~~text
anodrel-windows-host --native-window-controls-template-client <client.exe>
~~~

The host creates one direct Windows session and grants exactly:

- `ui.document.write`;
- `ui.events.read`;
- `window.title`;
- `window.state`;
- `window.focus`;
- `window.fullscreen`;
- `window.size`; and
- `session.close`.

It owns the native view, validated display-name suffix, direct User32 calls,
fullscreen restoration data, UI-thread command bridges, process, pipe worker,
and shutdown. The generated executable has no route to choose or inspect those
values. Every other native development template retains its current fixed grant
set.

## Automated verification

Automated coverage proves all of the following:

- command parsing, isolated project output, README route, and strict generated
  document shape;
- generated source uses all five typed controls but has no raw request,
  window target, handle, protocol selector, or configuration input; and
- a real invited-pipe generated child submits each fixed document, sends the
  title, state, focus, fullscreen, and size requests through their matching
  host mailboxes, closes only its own session, and exits cleanly.

## Quick desktop check

On Windows, double-click `start-window-controls-template.bat` in the repository
root. It creates a uniquely named temporary project, builds it from the local
checkout, and opens it through the fixed development route. Advance the visible
actions in order: **Set host-composed title**, **Resize client area**,
**Maximise window**, **Restore window**, **Request foreground attention**,
**Enter fullscreen**, **Return to windowed**, and **Complete window-controls
session**.

Confirm that the caption keeps the host-owned Anodrel suffix, the size changes
before maximising, maximise and restore affect only this window, fullscreen
returns to its framed presentation, and the session closes cleanly. A focus
request is accepted without focus readback; Windows' own policy controls its
visible outcome. The helper creates no certificate, package, installer,
application record, or machine policy and does not make the executable a
product application.

See Decision 0105, `docs/WINDOW_TITLE.md`, `docs/WINDOW_STATE.md`,
`docs/WINDOW_FOCUS.md`, `docs/WINDOW_FULLSCREEN.md`, and `docs/WINDOW_SIZE.md`.
