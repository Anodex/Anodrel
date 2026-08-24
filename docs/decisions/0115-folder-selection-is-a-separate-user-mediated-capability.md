# Decision 0115: Folder selection is a separate user-mediated capability

**Status:** Accepted

**Date:** 2026-08-24

## Context

The existing dialog boundary lets an authenticated session select one file or
one save destination, each behind its own grant and portable value type. A
folder is not a file: it has no extension filter, does not prove that any child
is safe to read or write, and is often the root of a broad filesystem request.

Reusing `dialog.open_file`, returning an ordinary selected-file path, or adding
an application-selected folder mode would blur those boundaries. Giving a
selected folder a retained capability would make a modal picker silently become
a general filesystem API.

## Decision

Add `dialog.open_folder` at Protocol 1.28 behind a distinct host-issued
`dialog.open_folder` grant. Its payload is the exact empty object. It returns
only cancellation or one bounded `SelectedFolderPath`, encoded as a selected
absolute path. The path is display data only; it is not a folder reference,
handle, permission, or later filesystem authority.

`SelectedFolderPath`, `FileDialogSelection::Folder`, and
`FileDialogRequestKind::OpenFolder` remain distinct from their file and save
counterparts. The existing one-request `FileDialogMailbox` sends the request to
the host UI thread, accepts only a matching folder response, and keeps the
same timeout and no-queue rule.

On Windows, use the Common Item Dialog's documented folder mode with a
filesystem-only result. The direct adapter owns COM initialization, dialog and
shell-item release, result-buffer release, and conversion to the portable
value. It returns one closed unavailable category for cancellation-independent
native failures.

## Consequences

- Applications can request a familiar native folder choice without a browser
  runtime, a third-party dialog toolkit, or direct operating-system access.
- A folder result cannot be passed accidentally to the file-selection or
  selected-output paths.
- The host keeps dialog ownership, native state, result conversion, and later
  filesystem authority decisions private.
- Installed policy, SDK, mock-host, protocol, and Windows routing each need
  explicit compatibility coverage before the feature is complete.

## Revisit conditions

Revisit before adding an initial folder, title, filters, multiple selection,
folder access, directory enumeration, writing, a retained folder reference,
drag-and-drop, callbacks, background UI, a non-Windows adapter, packaging, or
production identity. Each would broaden a capability or native-authority
boundary and requires its own contract and threat-model review.
