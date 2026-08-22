# Decision 0094: Development native multi-window templates keep window authority fixed

**Status:** Accepted

**Date:** 2026-08-21

## Context

Protocol 1.25 now gives an authenticated session a bounded way to create,
update, receive semantic input from, and close its own secondary views. The
portable group, authenticated transport, installed-record policy, direct
Windows host, TypeScript SDK, and mock host already enforce that boundary.

The existing generated native UI project deliberately has only three grants:
document replacement, primary-view action reads, and whole-session close. The
separate menu template added one explicit fixed menu grant rather than silently
giving every generated program more authority. Adding `window.open` and
`window.close` to either existing route would make a new project appear to
have window-management authority by default. Letting the generator accept a
capability list, title, size, position, window count, application identity, or
host command would undermine the same explicit boundary that Protocol 1.25
exists to protect.

## Decision

Keep `init`, `init-menu`, `--native-template-client`, and
`--native-menu-template-client` unchanged. Add a distinct,
operator-selected development path:

- `anodrel-native-app-tool init-multi-window <destination> <project-slug>
  <display-label>`; and
- `anodrel-windows-host --native-multi-window-template-client <client.exe>`.

The host creates one fresh development session with exactly these fixed grants:

- `ui.document.write`;
- `ui.events.read`;
- `window.open`;
- `window.close`; and
- `session.close`.

The route creates a `UiWindowGroup` from its host-created primary mailboxes and
uses the same private Windows group lifecycle as an installed session. The
generated child receives neither a native window handle nor a host mapping. It
can observe only the opaque secondary identity returned after the host created
and registered the view. The host fixes the application ID, session ID, primary
caption, secondary-caption suffix, process lifetime, pipe worker, and cleanup.
The selected executable remains unverified development code, not a package,
installed record, product launch, or signing mechanism.

Extend the preview `anodrel-ui-client` facade with a closed Protocol 1.25
surface:

- `open_window_v1` accepts one locally validated title and strict v1 document,
  returns only an opaque `SecondaryWindowId` after a validated response;
- `replace_window_document_v1` accepts only a `SecondaryWindowId` previously
  returned by the facade plus one locally validated strict v1 document;
- `read_window_actions` returns a bounded batch of revision-bound document
  actions tagged with their parsed logical `main` or secondary identity; and
- `close_window` accepts only `SecondaryWindowId`, so the template cannot
  express a close request for `main`.

The existing targetless `replace_document_v1` and `read_actions` remain the
ordinary primary-view methods. The facade exposes no raw operation escape
hatch, identity parser or constructor, enumeration, title readback, native
handle, window geometry, menu, dialog, callback, background receiver, or
capability input.

The generated source is one fixed visual walkthrough. A primary action opens a
secondary view; the secondary's first action replaces only that view's document;
its second action produces a tagged event, requests that exact secondary close,
then closes the authenticated session. The source does not read a document,
window ID, title, configuration, or command from a file, URL, environment,
argument, network connection, or native resource.

## Consequences

- Developers can build and visibly exercise the complete first-party
  multi-window API without Node.js, a webview, raw protocol JSON, or copied
  host wiring.
- Window capability remains an explicit choice of generator command and host
  route; regular and menu templates preserve their narrower grants.
- The facade makes the primary anchor and secondary-only close rule hard to
  misuse in the generated Rust source.
- The host has a real interactive development route that exercises the same
  private registration, title composition, timer servicing, and group shutdown
  path as Protocol 1.25 rather than treating the Group Lab as product evidence.

## Revisit conditions

Revisit before permitting a caller-selected capability, title, document,
window count, application identity, native setting, template asset source,
multi-window menu or dialog routing, product-session use, signing, packaging,
installation, a stable published native SDK, another operating-system adapter,
or background/concurrent event delivery. Each alters either the generated-code
authority or the host lifecycle boundary and requires its own decision.
