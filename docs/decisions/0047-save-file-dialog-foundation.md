# Decision 0047: Save-file selection stays separate from writing

## Context

Desktop applications need an operating-system save picker, but a picked
destination must not implicitly create, overwrite, or grant write access to a
file.

## Decision

Anodrel adds a bounded portable `SaveFilePath` and a direct Windows save picker.
The Windows adapter uses the host-selected owner, strict filters, required
existing parent directory, and the native overwrite confirmation prompt. It
returns only a chosen absolute destination or cancellation. No protocol surface
or file write service is added in this decision.

## Consequences

- The native picker warns before an existing file would be replaced.
- Selection alone cannot mutate the filesystem.
- A future authenticated `dialog.save_file` operation and a separate write
  service must define their own scoped authority and race behavior.
