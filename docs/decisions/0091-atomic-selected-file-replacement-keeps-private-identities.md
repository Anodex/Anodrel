# Decision 0091: Atomic selected-file replacement keeps private identities

**Status:** Deferred

**Date:** 2026-08-21

## Context

Protocol 1.17 and Protocol 1.22 deliberately use one retained Windows output
object for bounded, non-atomic text and binary writes. They never reopen a
returned path, but a failure after an in-place write begins can leave the
selected file partly changed.

The public [ReplaceFileW documentation](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-replacefilew)
describes a path-based helper with distinct failure outcomes in which names can
move independently. It cannot both retain Anodrel's selected object against a
replacement race and supply the simple all-or-nothing visibility boundary this
new operation needs.

Windows instead documents a handle-based rename through
[SetFileInformationByHandle](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-setfileinformationbyhandle)
and `FILE_RENAME_INFO`. Its `FileRenameInfoEx` form can use a retained parent
directory handle and the `FILE_RENAME_REPLACE_IF_EXISTS` plus
`FILE_RENAME_POSIX_SEMANTICS` flags. The latter keeps existing target handles
valid while later name resolution reaches the replacement. This is the narrow
primitive needed for a private selected-target transaction.

## Decision

Do not add an atomic replacement operation, reference type, capability, or
record version yet. The required future contract would need to preserve all of
these properties together:

- retain the selected existing regular-file identity against a replacement
  race until commit;
- stage complete private bytes and atomically switch only the selected name;
- leave an absent selected name absent when staging or commit fails; and
- avoid a path-based reopen, a target-sharing relaxation, or an in-place
  fallback.

The direct Windows experiment on this development machine established that the
documented `SetFileInformationByHandle` route can atomically create an absent
leaf from a private staged file, but cannot replace a target held without
delete sharing: it returns a sharing violation even with
`FILE_RENAME_REPLACE_IF_EXISTS | FILE_RENAME_POSIX_SEMANTICS`. Releasing that
target protection, or opening it with delete sharing, would let another process
replace the selected name before commit. `ReplaceFileW` has the same
path-identity problem as well as its documented multi-step failure cases.

The adapter experiment was removed rather than retained as dormant code. The
existing Protocol 1.17 and 1.22 one-use writers remain deliberately non-atomic.
No document may claim an atomic write, `atomicSaveReference`, or Protocol 1.25
until a supported direct Windows primitive can meet every requirement above and
is proved against real replacement races.

## Consequences

- Existing Protocol 1.17 and 1.22 writers remain the only output operations and
  explicitly retain their non-atomic failure semantics.
- A path-based, target-sharing, or in-place approximation cannot be added under
  the label “atomic replacement.”
- A future supported primitive, metadata policy, durable guarantee, multi-file
  transaction, recovery mechanism, binary boundary, or non-Windows adapter
  requires a new decision and a fresh implementation gate.

## Revisit conditions

Revisit only after identifying a supported direct Windows primitive that can
atomically name a staged file over a retained selected identity without opening
the selected target to replacement. Then reassess binary content, a generic
write mode, a path or folder input, caller-selected temporary naming, metadata
or ACL preservation claims, a durability result, retry/progress, multiple
targets, persistent grants, recovery, another operating-system adapter, or
packaging-dependent behavior.
