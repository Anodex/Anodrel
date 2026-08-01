# Decision 0046: Open-file dialogs use a dedicated session capability

## Context

The portable values, Windows adapter, and UI-thread bridge make it possible to
expose one file picker to an authenticated application. The resulting selected
path is sensitive application data, but selecting it must not silently provide
filesystem authority.

## Decision

Protocol 1.7 adds `dialog.open_file`. It accepts one bounded structured filters
array and requires the host-issued `dialog.open_file` capability immediately
before the service request. Its only successful results are cancellation or one
selected absolute path. The service uses the bounded `FileDialogMailbox`, so
the pipe worker waits while the host UI thread calls the Windows adapter.

The operation gives no file read, write, directory, save, process, window, or
initial-directory capability. It returns `dialog.unavailable` without a native
error or path when the host cannot complete the request.

## Consequences

- Applications can request a user-mediated file choice without direct OS APIs.
- A selected path requires a later, separately granted file-access operation
  before it can be opened by the platform.
- Modal request concurrency remains one per authenticated session.
- Non-Windows hosts report the stable unavailable result until their own direct
  UI-thread adapter is implemented.
