# Decision 0125: Linux state storage uses private directory descriptors

- Status: Accepted
- Date: 2026-08-30

## Context

Anodrel's portable storage contract already limits an application to one
bounded opaque state snapshot. Windows implements that contract with a
host-derived directory tree, fixed staging and backup names, and atomic
replacement. Linux has the same identity-derived current-user layout, but a
generic path-based writer could follow a symbolic link, accept a substituted
file, or leave a partial value after an interrupted replacement.

The Linux adapter must keep the public storage contract unchanged while
enforcing the filesystem properties that make the one-snapshot guarantee
meaningful. It must remain a first-party direct operating-system adapter with
no bundled filesystem, database, or directory runtime.

## Decision

Add a Linux-only state-store adapter behind `anodrel-storage`. Its constructor
derives the application data location from the effective-account root supplied
by `anodrel-linux-paths` and the existing portable identity layout. It accepts
no caller path, directory, file name, mode, temporary name, or snapshot
recovery choice.

The adapter opens the effective home directory component by component from
`/` with Linux `open` and `openat` using `O_DIRECTORY`, `O_NOFOLLOW`, and
`O_CLOEXEC`. It treats any unavailable component, symbolic link, non-directory,
or malformed relative component as unavailable. It creates only the Anodrel
portion below the pre-existing account home:

~~~text
.local/share/Anodrel/Applications/<validated application ID>/data
~~~

Every newly created Anodrel directory uses mode 0700. Existing directories
from `Anodrel` downward must belong to the effective account and permit no
group or other access. The conventional `.local/share` ancestors remain
account-owned placement, not an Anodrel ownership claim.

The adapter opens only fixed `state.anodrel.v1`, `state.anodrel.v1.bak`, and
`state.anodrel.v1.stage` names through the opened data-directory descriptor.
Every state object must be a single-link regular file owned by the effective
account with no group or other permissions. A complete replacement writes and
syncs a new 0600 staging file, moves the prior state to the fixed backup,
moves staging into the current state location, and syncs the data directory
after each rename. Reading prefers a valid current state and falls back to one
valid backup; staging is never readable state. Clear removes only those fixed
regular files and syncs the data directory when it changed.

Failures return only the existing safe storage categories. They never reveal a
path, account, native status, file name, snapshot, recovery source, or
filesystem metadata. The adapter keeps one in-process operation lock. A
multiple-host-process writer policy remains deliberately unspecified and is
not inferred from this adapter.

## Consequences

- Linux gains a direct, recoverable, bounded state-store foundation without
  changing Protocol 1.10 or exposing any filesystem capability.
- Link and file-type checks happen through descriptors at the operating-system
  boundary, rather than after a path has been reopened.
- The first write performs bounded directory work only when the fixed Anodrel
  subtree is absent; later operations open a small fixed set of descriptors and
  read at most 256 KiB plus one byte.
- Existing permissive or foreign-owned Linux `Anodrel` directories fail closed
  rather than being silently adopted. Migration is separate work.

## Alternatives considered

**Use `std::fs::create_dir_all` and paths for every operation.** It cannot
express the required no-follow walk or bind the final names to one opened
directory. Refused.

**Trust a symbolic link inside the account home.** It would turn a
host-derived identity namespace into authority over a location selected by the
link target. Refused.

**Use SQLite or another storage library.** One bounded snapshot does not need
a database, and it would add a shipped runtime dependency. Refused.

**Synchronize or merge writes from multiple hosts.** That needs a separate
cross-process conflict, locking, and recovery policy. Refused for v1.

## Revisit conditions

Revisit before adding encryption, multiple host processes, migration from an
existing Linux directory, XDG configuration, scoped filesystem access, keys,
binary values, streaming, directory cleanup, a service-account path, or a
Linux product host.
