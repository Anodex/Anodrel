# Decision 0161: Owned release-bundle authoring is bounded and fresh-output-only

**Status:** Accepted

**Date:** 2026-08-31

## Context

The owned release codec can encode verified in-memory entries, but a release
operator needs a reproducible filesystem authoring path before a manifest and
resource-bearing installer image can be assembled. ZIP, CAB, and third-party
archiver tooling violate the project's ownership boundary.

## Decision

Provide a separate first-party `anodrel-release-bundle-tool` with one command:
`create <source-directory> <new-bundle>`. It accepts only an existing absolute
normal source directory and a previously absent absolute output file. It walks
normal directories without links, accepts regular UTF-8-named files only,
enforces the format file-count and total-size bounds while reading, derives
forward-slash relative paths, and sorts them by raw UTF-8 bytes before encoding.

The tool parses the encoder result again before it creates the output file. It
writes with create-new semantics and synchronizes the one new file; an error
removes only that file. It cannot mutate source input, overwrite an output,
embed resources, select a certificate, sign, install, launch, or download.

## Consequences

- Release bundles are reproducibly authorable with Anodrel code and no archive
  executable, compression library, or installer framework.
- Link-like and special filesystem entries cannot silently become release files.
- The tool remains an authoring boundary; Windows staging still owns later
  device-name and install-path checks.

## Revisit conditions

Revisit for a measured need for streaming authoring, compression, file metadata,
cross-platform link semantics, or a different product package format.
