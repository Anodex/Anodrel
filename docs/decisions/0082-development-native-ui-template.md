# Decision 0082: Development native UI templates use a typed constrained client

**Status:** Accepted

**Date:** 2026-08-21

## Context

The static `anodrel.text.v1` package template gives a new project a runnable
host-owned content surface, but it intentionally cannot execute application
code. Decision 0081 added the first-party native child transport foundation
and compiled diagnostics prove it through a real Windows pipe and UI session.
It deliberately stopped short of a public native SDK or application template.

Phase 3 still requires a new executable project to run without its author
copying bootstrap, framing, JSON envelopes, request IDs, or host orchestration.
Letting a template select arbitrary capabilities, configure a pipe endpoint,
write machine policy, or claim a product identity would turn a development
convenience into an unverified launcher and bypass the production signing and
packaging decisions.

## Decision

Add a small development-native-UI path in three independently testable parts.

`anodrel-ui-client` is a portable typed facade over an already authenticated
`anodrel-client` conversation. Its initial surface is closed:

- replace one strict `anodrel.ui.document.v1` string and return its validated
  nonzero revision;
- read one bounded batch of validated `ui.action.invoked` events; and
- request `session.close` for the current authenticated session.

The facade always uses Protocol 1.3, owns unique bounded request IDs, parses
only the documented result fields, and collapses malformed or unexpected
responses into closed errors. It has no raw operation method, application ID,
capability input, callback, background receiver, reconnect, log sink, native
handle, or operating-system dependency.

`anodrel-native-app-tool init` creates a new, otherwise empty directory
containing a small Windows native UI project, its README, and a copyable fixed
document/action example. It validates the project slug and display label before
writing, refuses an existing destination, calculates only relative paths to the
local Anodrel checkout, and escapes every generated Rust string. It installs
nothing, runs nothing, writes no machine policy, signs nothing, and never
accepts a capability list. The generated project uses only Anodrel crates, the
Rust standard library, and direct Windows APIs.

The direct Windows host gains one explicit development command that receives an
operator-selected compiled template executable. It creates one host-owned UI
session with exactly `ui.document.write`, `ui.events.read`, and `session.close`;
it owns the window, input mailbox, pipe worker, child lifetime, and cleanup.
The application has no window target, title, menu, file, clipboard, network,
credential, storage, notification, process, or machine-policy capability. The
template is unverified development code: it is not a package format, installed
application record, product launcher, or replacement for the verified product
session.

The boundary is:

~~~text
generated native application
       |
anodrel-ui-client (typed UI session only)
       |
anodrel-client + anodrel-windows-client (private invited stream)
       |
Windows development-template host session (three fixed grants)
~~~

## Consequences

- A new native executable project can exercise an owned Anodrel window without
  Node.js, a webview, or host-source knowledge.
- Bootstrap handling, protocol envelopes, response parsing, and polling stay in
  one first-party implementation rather than being copied into project files.
- The generated project demonstrates one concrete narrow UI session; it does
  not imply that all native applications have production identity or broad OS
  authority.
- The typed facade is a documented preview surface in the repository, not a
  separately published stable SDK package. Its initial compatibility is proven
  by unit, real-pipe, generated-project, and host-boundary tests.

## Revisit conditions

Revisit before adding v2 scroll documents, menus, fields, arbitrary protocol
requests, application-declared capabilities, asynchronous/concurrent calls,
cross-language support, non-Windows launch adapters, a published crate, signed
templates, packaging, installation, updates, or production executable identity.
Each changes the native application boundary and needs its own contract and
threat-model review.
