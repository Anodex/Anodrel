# Decision 0150: Recovery removes only normal private stage trees

**Status:** Accepted

**Date:** 2026-08-31

## Decision

The owned recovery cleanup consumes only candidates discovered by Decision
0149. Before removing a candidate it checks that the root is a normal directory,
then enumerates its children with direct Kernel32 APIs. It refuses every reparse
point instead of following it, deletes normal files, and removes directories
only after their checked children are gone.

The operation accepts only an installer-selected application root and returns a
count. It does not accept arbitrary deletion paths, delete version directories,
read or write registry policy, select an application, launch a process, or
delete a stage whose name is merely similar to the private format.

## Consequences

- A crash during pre-promotion staging is recoverable without broad deletion.
- Junctions and symbolic links cause safe failure rather than traversal.
- A partially removed stale stage remains unselected and can be retried later.
