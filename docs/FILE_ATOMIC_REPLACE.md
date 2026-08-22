# Anodrel atomic selected-file replacement

**Status:** Deferred. The current direct Windows host implements only the
separate non-atomic writers documented in `docs/FILE_WRITE.md` and
`docs/FILE_BINARY_WRITE.md`.

## Purpose

This document records the requirements for a future bounded text-only atomic
replacement. It does not add an operation, capability, protocol version, or
reference type. A save dialog result, an absolute path, a folder, a native
handle, or an old `saveReference` remains unable to request atomic output.

## Required semantics

Before this feature can be proposed again, a direct Windows implementation must
retain the selected existing file identity against replacement, privately stage
and flush complete bytes, atomically switch only that same selected name, and
leave an absent selected name absent when the operation fails. It must not
reopen a path, weaken the retained target's sharing protection, use an in-place
fallback, or expose a temporary path, target handle, directory handle, native
error, metadata, or retry surface.

The first direct experiment held a selected target, an immediate non-reparse
parent directory, and a CNG-random sibling stage. It proved staged creation of
a still-absent name, but Windows rejected a replacement of the retained target
with a sharing violation. Permitting delete sharing would make the final name
replaceable before commit; using a full path loses the retained directory
identity; and `ReplaceFileW` is path-based with documented intermediate
failure states. The experiment was removed.

Consequently, no `dialog.save_file.v3`, `file.replace_text_atomic`,
`atomicSaveReference`, or `file.replace_text_atomic` grant exists today.

## Security rules

- `file.write_text` and `file.write_binary` remain their documented,
  non-atomic retained-output operations.
- No application may select an atomicity mode, temporary location, target
  path, directory, overwrite policy, handle, offset, or metadata behavior.
- A future implementation must reject unsupported storage conditions rather
  than weaken itself to a path-based or in-place write.

See `docs/PROTOCOL.md`, `docs/THREAT_MODEL.md`, and Decision 0091.
