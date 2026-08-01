# Decision 0045: Modal file dialogs cross through a bounded UI-thread bridge

## Context

An authenticated pipe session runs its request processing on a worker thread.
Windows common dialogs are native UI and must remain owned by the host UI
thread. Allowing the pipe worker to call Comdlg32 would violate Anodrel's
threading boundary and give a background protocol handler ambient window UI
authority.

## Decision

The portable file-dialog crate provides a `FileDialogMailbox`. It permits one
pending or displayed request per session, passes only strict filters to the UI
thread, and waits no longer than two minutes for a selected path, cancellation,
or safe failure. A request has no native owner handle, initial directory, raw
flags, filesystem access, or protocol authority. Only the host UI thread takes
and completes it; a second request fails while the first is active.

The Windows adapter stays a small UI-thread-only Comdlg32 call. The public
protocol capability is added only after an authenticated session is wired to a
real native message loop through this bridge.

## Consequences

- A worker cannot open a Windows dialog directly.
- Modal dialog concurrency is bounded and deterministic.
- A selected path remains a value, not a filesystem capability.
- Hosts must explicitly integrate the bridge; unused hosts expose no dialog
  service.
