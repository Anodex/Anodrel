# Anodrel development native scroll-window template

**Status:** The constrained generator, fixed Windows host route, generated-child
authenticated walkthrough, and one-click desktop launcher are implemented. The
documented visual Windows walkthrough remains pending. This is a Windows
development template, not a product packaging or application-identity format.

## Purpose

The scroll-window template creates a small first-party Rust executable that
visibly walks through Protocol 1.27's explicit secondary scroll-document
surface. A primary action opens a strict v2 secondary view. The person scrolls
that host-owned view to reach a semantic action; the program then replaces only
that secondary view with another strict v2 document. A final local action closes
that exact secondary and then the authenticated session.

It does not let the generated application control or observe a scroll offset,
native input, scrollbar, accessibility provider, window geometry, native
handle, capability grant, policy, bootstrap record, wire frame, package,
identity, or host lifecycle.

## Generator contract

`anodrel-native-app-tool init-scroll-window <destination> <project-slug>
<display-label>` accepts the same validated destination, Cargo-compatible
project slug, and bounded display label as the existing generators. It refuses
an existing destination and writes only:

~~~text
my-native-scroll-window-app/
|- Cargo.toml
|- README.md
`- src/
   `- main.rs
~~~

Every Anodrel dependency path is relative to the local checkout. The generator
does not install, run, sign, package, register, trust, or assign identity to
the generated executable. It accepts no capability list, native handle, window
ID, title, size, position, monitor, scroll position, scrollbar setting, menu,
host command, document path, URL, or configuration value.

The generated source contains only a fixed staged walkthrough. Its primary
document is strict v1 because it only provides the opening action. Both
secondary documents are strict v2 and contain one overflowed root `scroll`
node. Their semantic actions sit below the initial viewport, so local native
scrolling is necessary before each next step.

## Typed client contract

The generated source uses only these closed `anodrel-windows-ui-sdk` methods:

| Method | Input | Typed result | Protocol | Required grants |
| --- | --- | --- | --- | --- |
| `open_window_v2` | title plus one strict v2 document | `SecondaryWindowId` | 1.27 | `window.open`, `ui.document.write` |
| `replace_window_document_v2` | received `SecondaryWindowId` plus one strict v2 document | `DocumentRevision` | 1.27 | `ui.document.write` |
| `read_window_actions` | none | bounded tagged action batch | 1.25 | `ui.events.read` |
| `close_window` | received `SecondaryWindowId` | accepted close request | 1.25 | `window.close` |
| `close` | none | accepted group close | existing | `session.close` |

`SecondaryWindowId` is opaque and only a successful v2 opening response can
produce one. It is not a native handle, global identity, title, or scroll
selector. `close_window` cannot express `main`. Tagged actions expose only the
logical view identity, nonzero document revision, and semantic action ID. A
group pull returns at most 128 tagged actions: at most 32 from each of four
host-owned views. It carries no pointer, keyboard, scroll, focus, geometry,
close reason, native state, or accessibility information.

The two v2 methods locally reject malformed or oversized documents and the
opening method rejects an invalid title before any request is sent. The host
independently remains authoritative.

## Development host session

The implemented command is:

~~~text
anodrel-windows-host --native-scroll-window-template-client <client.exe>
~~~

The host creates one direct Windows session and grants exactly:

- `ui.document.write`;
- `ui.events.read`;
- `window.open`;
- `window.close`; and
- `session.close`.

It creates and owns the primary view, secondary native mapping, captions,
geometry, scroll state, scrollbar, pointer and keyboard input, UI Automation
scroll behavior, process, pipe worker, and group shutdown. The child cannot
select or inspect those values. Ordinary, menu, form, live-status, and v1
multi-window development routes retain their current fixed grant sets.

## Automated verification

Automated coverage proves all of the following:

- command parsing, isolated project output, and generated README route;
- locally valid v1 primary and strict v2 secondary documents;
- no raw protocol, position, host command, or accidental v1/v3 secondary path
  in generated source;
- a real invited-pipe generated child opens the v2 secondary, receives only
  the expected tagged actions, replaces only that view at revision 2, closes
  that issued identity, closes its group, and exits cleanly; and
- the fixed host route uses exactly the five listed grants and its private
  multi-window lifecycle.

## Quick desktop check

On Windows, double-click `start-scroll-window-template.bat` in the repository
root. It creates a uniquely named temporary project, builds it from the local
checkout, and opens the executable through the fixed development route. In the
**Anodrel Native Scroll Window Template** primary window, activate **Open
scrollable secondary window**. In the resulting secondary window, scroll until
**Reveal updated scroll document** appears and activate it. Scroll the
replacement document until **Close secondary and finish** appears and activate
it. A successful close prints a completion message and leaves the disposable
source project in the temporary directory it reports.

The helper creates no certificate, package, installer, application record, or
machine policy. It is a convenience for this development check; it neither
runs the generated executable directly nor turns it into a product application.

The remaining manual Windows check requires opening the primary, opening the
secondary, scrolling locally until the first action appears, activating it,
scrolling the replacement document to its final action, activating it, and
confirming the secondary and then the session close. It must not claim a
product launch or imply application control of native scroll state.

See Decision 0103, `docs/SCROLLING.md`, and `docs/MULTI_WINDOW.md`.
