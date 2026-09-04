# Decision 0194: Development native file-write templates keep output retained

**Status:** Accepted

**Date:** 2026-09-03

## Context

Protocols 1.17 and 1.22 already implement a secure selected-output flow: the
host opens the save dialog, captures one native output object, returns an
opaque one-use `saveReference`, and lets a separately granted writer consume
that retained object. The older development diagnostic needs a Node.js client.

Adding save selection or file writing to an existing template would silently
widen unrelated executable authority. A generator that accepted a path,
filename, extension, output text, write mode, reference, or atomicity choice
would weaken the host-owned selection and retained-identity boundary.

## Decision

Keep every existing generator command and host route unchanged. Add one
operator-selected development path:

- `anodrel-native-app-tool init-file-write <destination> <project-slug>
  <display-label>`; and
- `anodrel-windows-host --native-file-write-template-client <client.exe>`.

The host creates one development session with exactly:

- `ui.document.write`;
- `dialog.save_file`;
- `file.write_text`; and
- `session.close`.

The generated program has one compiled-in document, one compiled-in
`Text documents`/`txt` filter, and one compiled-in short UTF-8 text value. It
handles only selected or cancelled output, writes only through the opaque
reference the host returned, and then requests only its own session close.

It has no caller-supplied path, initial directory, filename, filter syntax,
reference, native handle, binary data, append mode, offset, stream, progress,
retry, atomicity, durability result, readback, event reader, configuration,
network access, package identity, installer, or signing behavior.

## Consequences

- Developers can visibly exercise the existing direct Windows retained-output
  path without Node.js, raw protocol JSON, or an external UI library.
- Existing templates retain their smaller fixed grant sets.
- The generated-child integration test proves private transport, strict filter
  selection, opaque reference handover, one text write, and self-close using
  an in-memory host service. A real save dialog and selected-file contents
  remain the manual Windows acceptance check.

## Revisit conditions

Revisit before adding configurable filters, caller text, binary output,
multiple selections, writes after cancellation, a path or filename input,
append, offsets, streaming, atomic replacement, durability reporting,
readback, persistent permission, non-Windows adapters, product identity,
packaging, or signing. Each changes authority, privacy, or lifecycle and
needs its own contract, threat-model update, and verification.
