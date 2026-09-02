# Decision 0178: Initial-install consent is native and one-shot

**Status:** Accepted

**Date:** 2026-09-02

## Context

The initial-install preflight can prove that a current signed release has no
selected machine policy, but it must not use that fact to prompt for UAC or
write machine state without a person's explicit decision. A generic dialog or
application-provided text would make a security-sensitive system prompt look
like application UI and let untrusted content influence an elevated route.

## Decision

Use direct Windows `MessageBoxW` to show one fixed confirmation only after an
opaque `PreparedInitialInstall` has passed the signed release and missing-policy
gate. Display only that signed release's package version. Use `Yes` and `No`,
with `No` as the default. Approval returns a distinct opaque value that keeps
the original prepared candidate and is required by the later fixed UAC handoff.

The adapter accepts no text, title, owner window, application identity, path,
publisher, certificate, package, capability, preference, or installation
argument. It neither elevates nor starts an installation.

## Consequences

- The later UAC boundary has an explicit human-decision input but no ability to
  choose an installer command or target.
- A cancellation leaves the machine unchanged and has no retained preference.
- The first console-installer slice owns a small native dialog before a future
  branded installer shell exists.

## Revisit conditions

Revisit for an owned installer window, localized product copy, an owner-window
policy, accessibility review of the installer surface, alternate installation
scope, a managed deployment route, another platform, or a restart coordinator.
