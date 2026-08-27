# Decision 0116: Folder-entry access uses one retained folder identity

**Status:** Accepted

**Date:** 2026-08-27

## Context

Decision 0115 intentionally makes `dialog.open_folder` a display-only choice.
That prevents a selected path from silently becoming broad directory authority,
but leaves no safe way for an application to use a folder a person chose.

Accepting an application-supplied path, extending the original picker result,
or keeping a reusable folder grant would permit arbitrary filesystem probing,
replacement races, recursive access, and permission persistence. Reusing a
file `SelectionReference` would blur directory and regular-file identity rules.

## Decision

Add `dialog.open_folder.v2` and `folder.read_entries` in Protocol 1.29.
The v2 picker retains the existing `dialog.open_folder` grant and returns the
same display path plus a new opaque `FolderReference` only after the host has
captured the selected folder's native identity. `folder.read_entries` requires
the distinct `folder.read_entries` grant, accepts only that reference, consumes
it once, and returns at most 32 immediate child names with a safe kind and a
complete flag.

The reference is 128 bits of host randomness encoded as exactly 22 unpadded
base64url characters. It is session-bound and distinct from selected-file and
save references. The portable layer manages only grammar and lifetime; the
operating-system adapter owns all folder handles, identity comparison, and
enumeration.

On Windows, capture rejects a selected reparse point, retains a directory
handle that prevents target replacement, rename, and deletion, and verifies a
private enumeration handle against the captured identity before it reads names.
An identity change, a reparse point, an unreadable item, or any native failure
returns only `folder.unavailable`.

## Consequences

- Folder choice and folder-entry access remain independently grantable.
- A selected path cannot be replayed or converted into a directory API.
- The first useful folder operation stays bounded, one-level, one-use, and
  free of recursive traversal, pagination, child paths, and write authority.
- Native identity code stays in a small adapter rather than entering protocol,
  renderer, or application modules.

## Revisit conditions

Revisit before adding pagination, recursion, child selection, file content,
writing, creation, deletion, rename, watches, drag-and-drop, persistent folder
permissions, multiple selection, non-Windows adapters, packaging, or
production identity.
