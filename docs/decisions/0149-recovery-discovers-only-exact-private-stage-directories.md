# Decision 0149: Recovery discovers only exact private stage directories

**Status:** Accepted

**Date:** 2026-08-31

## Decision

Recovery first discovers candidates without deleting them. It accepts one
installer-owned absolute application root, canonicalizes it, and returns only
normal directories whose names exactly match Anodrel's private staging format:
`.anodrel-stage-<major>-<minor>-<patch>-<process>-<sequence>`.

Version directories, unknown names, files, and link-like entries are not
candidates. This discovery boundary does not read registry policy, select an
application, remove a directory, launch a process, or expose candidates to an
application. A later direct Windows deletion boundary must consume only these
private candidates and refuse reparse points while walking their contents.

## Consequences

- Interrupted pre-promotion stages have a narrow recoverable identity.
- No broad recursive deletion is hidden inside scanning logic.
- Complete unselected version directories remain intact until a separate
  policy-backed cleanup decision exists.
