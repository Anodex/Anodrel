# Decision 0186: Product shortcuts follow completed policy transitions

**Status:** Accepted

**Date:** 2026-09-02

## Context

The direct Start-menu writer can create one verified link, but installation,
update, rollback, and uninstall change the machine policy that decides which
product that link represents. A link remembered only by an installer process
becomes stale after rollback; deleting a previous link before a replacement is
durable can hide a valid selected product. Reversing a valid policy transition
because a shell registration operation failed would also make a recoverable
product-surface problem alter the selected release.

## Decision

The installer captures only the prior selected record's optional signed
Start-menu name before update or rollback. After a completed policy transition,
it reads the newly selected record again and derives the canonical link solely
from that fresh proof. A newly declared link is written before a differing old
link is removed. A selected record without Start-menu metadata removes the
captured prior link, if any, and creates no replacement.

Initial installation has no prior selected policy, so it only synchronizes the
newly selected record after publication. Uninstall removes the currently
verified link before it removes policy or package files; a missing link is
ordinary, but a non-regular or undeletable link stops uninstall while policy
still selects the product.

Policy publication remains authoritative. If a post-publication registration
step fails, the selected release remains selected and the installer reports the
registration failure rather than claiming complete success or attempting a
policy rollback. Shortcut removal accepts only the fixed signed filename and
refuses a reparse point or non-regular file.

## Consequences

- A successful policy transition cannot point a new Start-menu entry at the
  wrong release.
- A transient stale entry is safer than removing the only working product
  entry before its replacement has been persisted.
- Legacy records that never declared a Start-menu name remain installable and
  do not acquire an inferred product link.
- Link registration remains a separate, reviewable post-policy stage with no
  application input, generic path, icon, arguments, AUMID, or activation.

## Revisit conditions

Revisit for multiple shortcuts, localized names, repair, package ownership,
Apps & features, AUMID activation, per-user installation, a restart manager,
or another operating system.
