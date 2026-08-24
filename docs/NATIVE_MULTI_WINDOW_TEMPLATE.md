# Anodrel development native multi-window template

**Status:** The typed multi-window facade, constrained generator, fixed Windows
host route, and real authenticated generated-child walkthrough are implemented
and automatically verified. The documented visual desktop walkthrough remains
pending. This is a Windows development template, not a product packaging or
application-identity format.

## Purpose

This template creates a small first-party Rust executable that visibly
walks through Protocol 1.25's bounded session-owned multi-window surface. It
will let a developer open a secondary view from a primary action, replace the
secondary document after a second action, receive the semantic event tagged
with the opaque view identity, request that exact secondary close, and end its
own session.

It will keep bootstrap records, wire frames, request IDs, raw JSON envelopes,
host policy, native window handles, and User32 lifecycle code out of generated
application source. It is separate from both `docs/NATIVE_UI_TEMPLATE.md` and
`docs/NATIVE_MENU_TEMPLATE.md`; neither existing template acquires window
authority because this one exists.

## Generator contract

`anodrel-native-app-tool init-multi-window <destination> <project-slug>
<display-label>` will accept the same validated destination, Cargo-compatible
project slug, and bounded display label as the existing generators. It refuses
an existing destination and writes only:

~~~text
my-native-multi-window-app/
|- Cargo.toml
|- README.md
`- src/
   `- main.rs
~~~

Every Anodrel dependency path is relative to the local checkout. The tool does
not install, run, sign, package, register, trust, or assign identity to the
generated executable. It does not accept a capability list, native handle,
window ID, title, size, position, monitor, menu, host command, or document
path.

The generated source contains a fixed, staged v1 document walkthrough. Its
only application-defined title is a fixed secondary proposal compiled into the
source. It will not read content, title, identity, configuration, or a document
from an argument, file, environment variable, URL, network connection, or
native resource.

## Typed client contract

`anodrel-windows-ui-sdk` exposes the following closed methods:

| Method | Input | Typed result | Protocol | Required grants |
| --- | --- | --- | --- | --- |
| `open_window_v1` | title plus one strict v1 document | `SecondaryWindowId` | 1.25 | `window.open`, `ui.document.write` |
| `replace_window_document_v1` | received `SecondaryWindowId` plus strict v1 document | `DocumentRevision` | 1.25 | `ui.document.write` |
| `read_window_actions` | none | bounded tagged document-action batch | 1.25 | `ui.events.read` |
| `close_window` | received `SecondaryWindowId` | accepted close request | 1.25 | `window.close` |

`SecondaryWindowId` is opaque and can be created only by a successful
`open_window_v1` response. It is neither a native handle nor an application
global ID. `close_window` accepts no `main` value; `close` remains the one
method that ends the full authenticated group. A tagged action exposes only a
logical `main`/secondary identity, the nonzero document revision, and the
semantic action ID. A group pull returns at most 128 tagged actions: the
maximum of 32 candidates from each of its four bounded views. It carries no
focus, pointer, key, geometry, close reason, or native state.

`open_window_v1` and replacement validate strict v1 documents locally before
sending them. Opening also validates the bounded no-control-character title.
The host remains authoritative and independently validates every request.
Malformed response identities, revisions, counts, tags, event shapes, and
unexpected menu events fail closed. The existing primary-only document and
event methods remain unchanged.

## Development host session

The implemented command is:

~~~text
anodrel-windows-host --native-multi-window-template-client <client.exe>
~~~

It will create one host-controlled Windows session and grant exactly:

- `ui.document.write`;
- `ui.events.read`;
- `window.open`;
- `window.close`; and
- `session.close`.

The host creates and owns the primary view, private logical-to-native map,
secondary captions, native geometry, registration, icons, timers, process
handle, pipe worker, and group shutdown. The child has no way to select or
inspect those values. The regular and menu routes retain their existing closed
grant sets.

## Quick desktop check

On Windows, double-click `start-multi-window-template.bat` in the repository
root. It creates a uniquely named temporary project, builds it from the local
checkout, and opens the generated executable through the fixed multi-window
route. In the **Anodrel Native Multi-Window Template** primary window, activate
**Open secondary window**. In the resulting secondary window, activate
**Replace this window**, then **Close secondary and finish**. A successful
close prints a completion message and leaves the disposable source project in
the temporary directory it reports.

The helper creates no certificate, package, installer, application record, or
machine policy. It is a convenience for this one development check; it does not
run a generated executable directly or turn it into a product application.

## Compatibility and verification

This work uses existing Protocol 1.25 `window.open`, `window.close`,
`ui.document.replace.window`, and `ui.events.read.window` operations. It adds
no wire frame or core protocol operation. Typed-facade tests cover local
document and title rejection, request versions and payloads, opaque secondary
identity parsing, primary-close type exclusion, tagged actions, malformed
responses, each 32-action view bound, and the full 128-action group bound. The
generator's real-pipe integration test builds a generated executable, delivers its private
invitation, supplies the fixed primary action, services the one portable
secondary creation handoff, checks the secondary's independent document
revisions, supplies its two tagged actions, observes the exact secondary close,
and verifies group self-close plus clean child exit. The Windows host's
fixed-grant lifecycle has unit coverage.

Remaining proof is the documented manual desktop walkthrough. It must show the
primary window first, create a separate secondary window after the primary
action, update only the secondary after its first action, close only that
secondary after its second action, then close the primary session. It must not
infer native geometry, enumerate windows, or claim a production launch.

See Decision 0094 and `docs/MULTI_WINDOW.md`.
