# Windows initial-install elevation handoff

**Status:** The fixed UAC launcher and process-lifecycle boundary are
implemented. The default interactive installer command, a completion surface,
restart handling, and signed end-to-end fixture verification remain separate
work.

## Purpose

The installer must not turn a native confirmation into an administrator process
through a caller-selected executable or command. This boundary accepts only the
opaque approval created by [initial-install consent](INSTALL_CONSENT.md). It
asks Windows to relaunch the current installer with the one fixed `install`
command, then retains the process handle for an off-UI-thread wait.

## Direct elevation

The adapter obtains the current process image from Windows through the standard
current-executable route and rejects a non-absolute result. It calls direct
Windows `ShellExecuteExW` with the literal `runas` verb and fixed `install`
argument. It supplies no working directory, owner window, shell class, file
path, command, environment, endpoint, package, policy, publisher, or
certificate input.

Windows presents its normal UAC experience. UAC cancellation is an ordinary
safe outcome. Once Windows returns a process handle, the caller must wait away
from a UI thread. A handle is closed exactly once when the process owner is
consumed or dropped.

## Completion

A zero child exit is not installation success. It permits only the existing
opaque [initial-install acceptance](INSTALL_ACCEPTANCE.md) postcondition proof,
which reloads the fixed machine policy and validates the selected signed
release. A nonzero exit has no proof route. An abandoned child is not killed or
interpreted; it independently performs the installer’s fixed signature and
machine-policy gates.

## Exclusions

This boundary does not display confirmation, create a default installer route,
download, install a certificate, choose an elevation target, show progress,
parse child output, write machine policy, recover files, restart applications,
or claim a user-visible completion result.

See [Windows installer](WINDOWS_INSTALLER.md), [initial-install consent](INSTALL_CONSENT.md),
[initial-install acceptance](INSTALL_ACCEPTANCE.md), and Decision 0179.
