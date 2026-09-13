# Decision 0222: Uninstall retirement cleans one verified rollback package

**Status:** Accepted

**Date:** 2026-09-12

**Supersedes:** Decision 0153's rule that cleanup can remove only the current
policy-removed package tree.

## Context

An owned update deliberately retains one lower-version package and its private
`previous` machine-policy record for explicit rollback. An uninstall removes
the selected current record and package, but that older rollback package can no
longer serve any purpose. Leaving it behind prevents bounded fixture retirement
and makes a completed uninstall look incomplete.

Directory enumeration cannot select a safe deletion target. The retained
package must stay private installer recovery state, not an administrator or
application-selected path.

## Decision

After selected-policy removal and successful removal of the selected package,
the signed cleanup helper may read exactly the fixed private `previous` record.
It removes its package only when all of these conditions hold:

- the record parses as the same signed application identity;
- its package is a direct canonical-version child of the fixed machine root;
- that version is strictly lower than the signed current release; and
- its executable's Authenticode leaf fingerprint matches both the private
  record and the current signed release publisher.

An absent `previous` record is normal. Any invalid record, unexpected path,
version, signature, or deletion error fails closed. The elevated fixed
`cleanup-cache` command repeats this same bounded retirement only when no
current policy is selected. It does not enumerate versions, remove a selected
release, use cache content as a target, remove data or credentials, or accept a
path from a command line.

## Consequences

- A completed uninstall removes both the current release and the one retained
  rollback release without a reboot.
- Interrupted cleanup can finish from a freshly verified signed recovery image.
- More than one prior version, migration from legacy restart cleanup, and
  generic package pruning remain out of scope.

## Revisit conditions

Revisit for a bounded multi-version retention policy, package repair, per-user
installation, a persistent privileged service, or another operating system.
