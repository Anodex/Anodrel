# Decision 0141: Keep release bundles owned, bounded, and uncompressed

**Status:** Accepted

**Date:** 2026-08-30

## Context

Decision 0140 places a release manifest and payload inside one signed Windows
installer image. The payload still needs a format that can preserve exact file
paths and contents without adding ZIP, CAB, MSI, compression, or an archive
library to Anodrel's dependency graph.

An archive parser is exposed to signed-but-fallible release bytes. It must reject
path traversal, duplicate names, integer overflow, trailing data, malformed
UTF-8, oversized content, and integrity mismatches before later Windows code
can create an installation directory.

## Decision

Define the first-party `anodrel.bundle.v1` binary payload. It stores a bounded,
strictly ascending sequence of regular file entries. Every entry carries its
relative UTF-8 path, exact byte length, SHA-256 digest, and raw bytes. The
bundle is uncompressed: the release manifest authenticates the whole payload,
and the bundle authenticates each extracted file.

The codec operates on borrowed payload bytes and returns indexed borrowed file
slices. It never performs file I/O, creates a directory, follows a link,
allocates a copy of an entry's contents, decompresses data, or trusts an entry
path as an operating-system path.

## Consequences

Positive:

- The release format is Anodrel code with no archive or installer framework.
- Bounded raw entries keep parsing predictable and avoid decompression attacks.
- Deterministic ordering makes a bundle reproducible from the same entry bytes.
- Per-entry verification supports safe later staged extraction without retaining
  a second copy of a large installer payload.

Tradeoffs:

- Version 1 releases may be larger than compressed archives.
- The format intentionally supports regular files only; directories are derived
  by a later installer from verified file paths.
- Streaming resource access and compression, if ever needed, require separate
  measured format and security decisions.

## Revisit conditions

Revisit when measured release sizes require compression, when an installer needs
streaming resource access, when file metadata becomes essential, or when an
operating-system package format replaces the owned installer route.
