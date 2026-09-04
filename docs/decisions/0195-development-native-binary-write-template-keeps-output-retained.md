# Decision 0195: Development native binary-write templates keep binary output retained

**Status:** Accepted

**Date:** 2026-09-04

## Context

Protocol 1.22 already provides a distinct bounded binary writer over the
retained selected-output object created by `dialog.save_file.v2`. Its existing
development diagnostic needs a Node.js client. The text template introduced by
Decision 0194 must not silently acquire binary authority, and a project that
chooses an output path, bytes, MIME type, encoding, stream, or write mode would
widen the boundary that Protocol 1.22 deliberately keeps closed.

## Decision

Keep all existing generator commands and host routes unchanged. Add one
operator-selected development path:

- `anodrel-native-app-tool init-file-binary-write <destination> <project-slug>
  <display-label>`; and
- `anodrel-windows-host --native-file-binary-write-template-client <client.exe>`.

The host creates one development session with exactly:

- `ui.document.write`;
- `dialog.save_file`;
- `file.write_binary`; and
- `session.close`.

The generated program has one compiled-in document, one compiled-in `Binary
files` / `bin` filter, and the one compiled-in canonical base64url value for
the fixed byte sequence `41 6E 6F 64 72 65 6C 00 FF`. It handles only selected
or cancelled output, writes only through the opaque reference the host returned,
and then requests only its own session close.

It has no caller-supplied path, initial directory, filename, filter syntax,
reference, native handle, binary input, MIME type, encoding option, append
mode, offset, stream, progress, retry, atomicity, durability result, readback,
event reader, configuration, network access, package identity, installer, or
signing behavior.

## Consequences

- Developers can visibly exercise Anodrel's existing direct Windows binary
  export path without Node.js, raw protocol JSON, or an external UI library.
- The text template retains only its smaller `file.write_text` authority.
- The generated-child integration test proves private transport, strict filter
  selection, opaque reference handover, one exact binary write, and self-close
  using an in-memory host service. A real save dialog and selected-file bytes
  remain the manual Windows acceptance check.

## Revisit conditions

Revisit before adding configurable bytes, filters, MIME type, caller text,
multiple selections, writes after cancellation, a path or filename input,
append, offsets, streaming, atomic replacement, durability reporting,
readback, persistent permission, non-Windows adapters, product identity,
packaging, or signing. Each changes authority, privacy, or lifecycle and needs
its own contract, threat-model update, and verification.
