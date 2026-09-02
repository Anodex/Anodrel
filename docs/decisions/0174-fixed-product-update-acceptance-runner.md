# Decision 0174: Product-update acceptance uses one fixed development identity

**Status:** Accepted

**Date:** 2026-09-02

## Context

Unit and contract tests establish the ordering and refusal behavior of the
native updater, but cannot prove a real HTTPS catalogue, CMS signature,
Authenticode image, UAC handoff, elevated installer, and final machine record
work together. A test runner that accepted an application ID, endpoint, path,
or installer argument would become an unreviewed general update launcher.

## Decision

Add `anodrel-product-update-acceptance`, an operator-only native diagnostic
with no arguments. It selects only the compile-time development fixture identity
`org.anodrel.product-fixture` and composes the existing signed discovery,
host-owned consent, private download, locked image acceptance, UAC handoff,
process observation, and postcondition proof in that order.

The initial interactive thread may show the fixed Anodrel confirmation. After
approval, the diagnostic has no product UI and may perform the transfer and
wait on its command-runner thread. A normal interactive host remains required
to use a worker for blocking work. The diagnostic reports only closed outcomes
and never turns an installer exit into acceptance without the existing policy
proof.

## Consequences

- The real update sequence has one repeatable, narrow acceptance entry point.
- No application, protocol, command line, or environment value can redirect it
  to another machine record or network location.
- Development still needs an intentionally provisioned newer signed fixture and
  an HTTPS catalogue before a positive run is possible.
- The diagnostic does not create a user-facing product update feature, restart
  policy, background work, scheduler, preference, or release channel.

## Revisit conditions

Revisit for a production acceptance runner, a distinct fixture identity,
automated isolated-machine testing, a host update UI, restart coordination,
automatic scheduling, a privileged broker, or a non-Windows platform.
