# Decision 0170: Update cache is fixed per installed application

**Status:** Accepted

**Date:** 2026-09-01

## Context

The image downloader intentionally requires a native-owned cache directory,
but leaving its selection to an eventual coordinator would permit accidental
reuse of an application path or an arbitrary temporary directory. Retained
images also need safe cleanup after an unobservable or abandoned handoff.

## Decision

Derive one update cache from the fixed machine record's validated application
identity and the existing current-user Local AppData layout:
`cache\updates` below that application namespace. Create its owned components
only as normal non-reparse directories.

The downloader's private filenames are exact `.anodrel-update-<pid>-<sequence>.exe`
spellings. Recovery scans only this selected directory and deletes only normal
files matching that bounded numeric grammar. It never recurses, follows a link,
removes a directory, forces a locked-file deletion, or interprets another
file as an updater artifact.

## Consequences

- Update artifacts are isolated by installed application identity without
  granting applications filesystem choice or access.
- A handoff that cannot prove its child finished leaves its file for a future
  constrained cleanup pass instead of racing an executing image.
- The image-lock gate remains mandatory before UAC; cache ownership alone is
  not execution authority.
- User consent, transfer composition, progress, automatic scheduling, and
  update success proof remain separate.

## Revisit conditions

Revisit for protected machine-owned caching, user-selectable storage, resumable
transfers, multiple concurrent images, a background service, another platform,
or a cache quota policy.
