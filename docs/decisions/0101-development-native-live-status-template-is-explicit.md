# Decision 0101: Development native live-status templates keep status delivery one-way

**Status:** Accepted

**Date:** 2026-08-21

## Context

Decision 0100 adds one visible semantic `status` node and a bounded Windows UI
Automation live-region event to exact version-3 session documents. The typed
native `anodrel-ui-client` facade already has the three v3 replacement methods,
but the generated native project templates only demonstrate version-1 documents.
That leaves developers to assemble an accessibility demonstration themselves,
which risks accidentally presenting a status as a private callback, notification,
or screen-reader detection surface.

Adding v3 status behavior to the ordinary generated UI template would silently
change its teaching goal. Letting a generator caller choose status text,
politeness, timing, capabilities, native event data, or listener behavior would
turn a constrained diagnostic into an uncontrolled accessibility sender.

## Decision

Keep the existing `init`, `init-form`, `init-menu`, and `init-multi-window`
templates unchanged. Add a separate, operator-selected development path:

- `anodrel-native-app-tool init-live-status <destination> <project-slug>
  <display-label>`; and
- `anodrel-windows-host --native-live-status-template-client <client.exe>`.

The host creates one fresh Windows session with exactly its ordinary UI grants:

- `ui.document.write`;
- `ui.events.read`; and
- `session.close`.

The generated project publishes three fixed complete v3 documents. The first
establishes a visible polite baseline. A person deliberately activates the first
semantic action to publish a distinct polite result; a second action publishes a
distinct assertive result; a final action closes the session. Each transition is
visible in the same document and has a new validated document revision. The
application receives only ordinary revision and semantic-action results. It has
no listener check, UI Automation identifier, notification route, announcement
result, focus readback, callback, native handle, capability input, configuration
loader, network access, package identity, or signing behavior.

The host fixes the application ID, session ID, caption, process lifetime, pipe
worker, and cleanup. The selected executable remains unverified development
code: it is not an installed record, product launcher, package, signer, or
production accessibility claim.

## Consequences

- A developer can build and visibly exercise v3 accessible status updates with
  only first-party Anodrel crates, the Rust standard library, and direct Windows
  APIs.
- The template proves that status delivery is outbound and best effort: the
  generated program progresses by a person's semantic actions, never by an
  accessibility response.
- Ordinary, form, menu, and multi-window templates retain their existing
  document versions and fixed authorities.
- A real-pipe integration test verifies the three exact v3 documents and their
  revision-bound actions without requiring Narrator. Narrator and Inspect remain
  the separate manual acceptance gate for actual announcement behavior.

## Revisit conditions

Revisit before adding caller-provided status text, application-selected
politeness, automatic timers, status history, notifications, accessibility
callbacks, listener inspection, more controls, another operating-system adapter,
packaging, production identity, or a stable published native SDK. Each changes
the accessibility, authority, or launch boundary and requires a new decision.
