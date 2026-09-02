# Decision 0177: Initial-install success requires policy proof

**Status:** Accepted

**Date:** 2026-09-02

## Context

The owned installer can carry out a fixed initial installation, but a process
returning zero alone cannot establish which application release machine policy
now selects. The update path already avoids this mistake with a postcondition
proof. An initial-install UI must not introduce a weaker definition of success.

## Decision

Add an opaque initial-install candidate that can be created only after the
current installer passes its signed embedded-release gate and its fixed
application identity has no selected machine record. Add a separate
postcondition proof for that candidate. It may run only after an elevated fixed
`install` process reports zero, then reloads the fixed record and requires exact
identity, selected-executable Authenticode, record publisher, installer
publisher, and canonical version continuity.

The candidate has no application protocol, UI, command, path, certificate,
registry, package, network, process, elevation, or installation input. The
proof has no restart, launch, cleanup, rollback, progress, notification, or
display behavior.

## Consequences

- A later native installer surface must place consent and UAC between these two
  opaque stages; it cannot call `install` and treat exit as installation proof.
- A selected existing policy refuses the initial route before any elevation.
- Initial installation and update share the same standard of claimed success.

## Revisit conditions

Revisit for a native installer window, a user-visible completion surface,
multiple installation scopes, repair, data migration, restart coordination,
rollback UX, a privileged broker, another platform, or a signed end-to-end
fixture run.
