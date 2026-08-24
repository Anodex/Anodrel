# Decision 0103: Development native scroll-window template keeps scroll state host-owned

**Status:** Accepted

**Date:** 2026-08-21

## Context

Protocol 1.27 gives a session-owned secondary view an explicit version-2
scroll-document opening and targeted replacement route. The direct Windows
host keeps each view's scroll offset, wheel and keyboard handling, scrollbar
input, UI Automation ScrollPattern, and ScrollItem behavior private. The
group lab proves that the native path can create such a view, but it is not a
small generated application a developer can build and run through the
authenticated invited-pipe path.

The existing native multi-window template deliberately demonstrates strict
version-1 documents. Replacing it with a scroll example, or giving the
ordinary native template the multi-window grants, would hide the document
format and capability boundary this feature is meant to prove. Letting a
generator accept content, a scroll position, scrollbar setting, window
geometry, identity, capability list, or host command would similarly make
private host policy appear to be application authority.

## Decision

Add a separate, operator-selected development template and host route:

- `anodrel-native-app-tool init-scroll-window <destination> <project-slug>
  <display-label>`; and
- `anodrel-windows-host --native-scroll-window-template-client <client.exe>`.

The host creates one fresh development session with the same fixed grants as
the existing multi-window template:

- `ui.document.write`;
- `ui.events.read`;
- `window.open`;
- `window.close`; and
- `session.close`.

The generated program has one fixed staged walkthrough. A strict v1 primary
document opens one strict v2 secondary scroll document. The person scrolls the
secondary using only host-owned native input to reach a semantic action. That
action causes the child to replace only that same secondary through
`replace_window_document_v2`; a second locally revealed action closes that
opaque secondary identity and then its whole authenticated session.

The typed client surface is limited to its existing explicit methods:
`open_window_v2`, `replace_window_document_v2`, `read_window_actions`,
`close_window`, and `close`. The generated source has no raw protocol escape
hatch, document input, configuration source, scroll-position field, native
handle, title readback, geometry, menu, dialog, network access, packaging, or
machine-policy behavior. The host supplies its private invitation only through
standard input.

## Consequences

- Developers can exercise the real Protocol 1.27 secondary scroll route using
  only first-party Rust, the standard library, and direct Windows APIs.
- Version-1 multi-window and ordinary templates retain their smaller document
  and authority contracts.
- Scrolling is visibly useful while its offset, input, accessibility control,
  and presentation stay where they belong: inside the native host.
- The generated-child integration test must prove v2 opening, independent
  secondary revisions, action tagging, v2 replacement, exact secondary close,
  whole-session close, and clean child exit. A person must still perform the
  documented desktop walkthrough.

## Revisit conditions

Revisit before accepting any application-selected document, scroll offset,
scrollbar behavior, title, capability, native setting, geometry, identity,
menu or dialog routing, product use, packaging, signing, installation,
background action delivery, a stable published native SDK, another operating
system adapter, or a second scroll-container relation. Each changes either the
fixed generated-code authority or the retained host-state boundary.
