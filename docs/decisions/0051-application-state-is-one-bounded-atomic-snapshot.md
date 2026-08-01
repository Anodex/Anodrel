# Decision 0051: Application state starts as one bounded atomic snapshot

**Status:** Accepted

**Date:** 2026-08-01

## Context

`anodrel-paths` derives an isolated per-application `data` location, but it
correctly does not create or expose it. Applications still need durable state.
Giving rendered content a general filesystem API would immediately introduce
path traversal, cross-application access, partial-write recovery, file-handle,
and enumeration policy without a narrow first use case.

## Decision

The first storage foundation represents one opaque UTF-8 snapshot, at most
256 KiB, for one host-validated application identity. The platform will own
its location below the existing `data` directory, atomic replacement, and
bounded recovery. The application owns only its snapshot schema.

The initial service is deliberately limited to reading the complete value,
replacing the complete value, and clearing it. It accepts no key, path,
filename, native handle, partial range, stream, enumeration request, or
application-controlled temporary file. Absence remains distinct from an empty
saved value.

The first implementation remains below the authenticated protocol boundary.
A later protocol revision must introduce independent host-issued capabilities
for reading, replacing, and clearing state, document exact values and safe
errors, and add contract tests before an application can call it.

## Consequences

- durable app state gains a small portable ownership model without a browser,
  database, or third-party runtime;
- a Windows adapter can concentrate direct filesystem and crash-recovery logic
  behind a single audited boundary; and
- applications retain freedom to version and migrate their own data format.

The tradeoff is that v1 cannot efficiently update one field, store files,
share state across processes, watch changes, or synchronize data. Those are
deliberate future decisions rather than hidden behavior.

## Revisit conditions

Revisit before adding keys, arbitrary binary data, a size increase, multiple
writers, directories, scoped document access, encryption or sync policy,
filesystem enumeration, streaming, or a public storage protocol operation.
