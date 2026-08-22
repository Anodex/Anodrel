# Decision 0095: Development native form templates keep typing host-owned

**Status:** Accepted

**Date:** 2026-08-21

## Context

The native UI document model already has a bounded single-line field, and
Protocol 1.15 already has the separately granted `ui.fields.read` snapshot
operation. Decision 0067 deliberately keeps keyboard input, caret state,
selection, edit history, and timing inside the host; an application may learn
only every current field value through one explicit whole-surface read.

The existing generated UI template cannot exercise that path. Giving its
three-grant route `ui.fields.read` would widen every generated application by
default. A general form generator that accepts arbitrary fields, source files,
templates, rules, or action commands would also obscure who owns the document
and tempt a developer to republish it while a person is typing.

## Decision

Keep `init`, `init-menu`, `init-multi-window` and their host routes unchanged.
Add a separate operator-selected development path:

- `anodrel-native-app-tool init-form <destination> <project-slug>
  <display-label>`; and
- `anodrel-windows-host --native-form-template-client <client.exe>`.

The host creates one fresh Windows session with exactly these fixed grants:

- `ui.document.write`;
- `ui.events.read`;
- `ui.fields.read`; and
- `session.close`.

The generated source publishes one fixed strict v1 form document containing
one enabled single-line `template.form.name` field and one
`template.form.submit` semantic action. It waits for that action at the
accepted document revision, then makes one `ui.fields.read` request. It accepts
only a closed, sorted snapshot carrying that one field ID, never logs, displays,
persists, forwards, or republishes the returned value, and then requests
self-close.

Extend the preview `anodrel-ui-client` facade with one closed `read_fields`
method. Its typed snapshot validates the protocol's 64-field bound, canonical
element IDs, single-line values no longer than the portable 4,096-character
field maximum, exact field object shape, and strict element-ID order. It has no
field selector, individual-field getter, changed flag, focus, caret, selection,
timestamp, native control, background receiver, raw protocol escape hatch, or
callback.

The host retains the document mailbox, field-read mailbox, current field
states, keyboard input, caret, selection, Win32 window, process, pipe worker,
and child lifecycle. The executable remains explicitly selected, unverified
development code rather than an installed application, product session,
package, signer, or identity.

## Consequences

- A developer can build and visibly test native text entry and intentional
  submit-time value readback without a webview, Node.js, or third-party runtime.
- The separate route makes the inward-facing `ui.fields.read` authority
  explicit; ordinary, menu, and multi-window templates remain narrower.
- The typed facade preserves the whole-surface, snapshot-only contract instead
  of making individual field reads convenient.
- Generated source demonstrates that form submission reads a value once; it
  does not establish live validation, password entry, stored forms, or a
  general form framework.

## Revisit conditions

Revisit before adding another field, application-provided form content,
caller-selected field IDs or labels, document replacement after input, value
display or persistence, validation messages, individual field selection,
change/event subscriptions, passwords, multi-line input, native handles,
another operating-system adapter, packaging, production identity, or a stable
published native SDK. Each changes the input-authority or launch boundary and
requires its own contract and threat-model review.
