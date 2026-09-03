# Decision 0185: Start-menu registration is an all-users atomic Shell Link

**Status:** Accepted

**Date:** 2026-09-02

## Context

Anodrel installation and selected policy are machine-wide. A per-user shortcut
would produce different product availability by profile and could not be
maintained as one machine-policy result. Writing a `.lnk` directly at its final
path can also leave a partial product surface if Shell Link persistence fails.

## Decision

Resolve the fixed `FOLDERID_CommonPrograms` Windows-known folder directly and
place exactly one link at `Anodrel\\<signed-start-menu-name>.lnk`. Require both
the known folder and the Anodrel child to be normal non-reparse directories.
Set only the selected executable target and selected package-root working
directory through `IShellLinkW`; do not set arguments, a custom icon,
description, source URL, or Application User Model ID.

Persist the link first to a system-created temporary ordinary file in that same
directory. Replace the final link only with same-directory `MoveFileExW`
replacement plus write-through. On an error, remove only that still-ordinary
temporary file. The writer is one no-argument operation behind fresh selected
policy proof.

## Consequences

- Every Windows user sees the same signed product entry after installer
  composition is added.
- A writer failure cannot intentionally leave a partly written final link.
- Future AUMID or activation work must extend the existing fixed link rather
  than invent another registration surface.

## Revisit conditions

Revisit for per-user installation, packaged identity, multiple product links,
icons, AUMID, activation, removal, desktop links, localization, or another
platform.
