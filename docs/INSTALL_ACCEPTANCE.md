# Windows initial-install acceptance

**Status:** The signed initial-install preflight and postcondition proof are
implemented. A later native installer surface must provide explicit consent and
direct UAC handoff before a person can use this as an interactive flow.

## Purpose

An elevated installer exit code is not enough to claim that its initial release
became the machine-selected application. Before elevation, the installer must
first prove that its own signed embedded release is valid and that no selected
record already exists. After a zero exit, it must independently prove that the
fixed machine policy selects the same signed release.

## Fixed proof sequence

~~~text
current signed installer + missing selected policy
                    |
                    v
opaque initial-install candidate
                    |
                    v
separate explicit consent and UAC handoff
                    |
                    v
elevated fixed `install` transaction exits zero
                    |
                    v
fixed machine record, executable signature, publisher, and version proof
~~~

The postcondition loads only the current installer manifest's fixed application
identity from machine policy. It requires the record and package to validate,
Windows to accept the selected executable's Authenticode signature, both the
record and current signed installer to match that signer, and the selected
package root to end in the same canonical version directory.

The resulting value is opaque. It does not report paths, registry data,
certificates, a process exit code, progress, restart behavior, or a launch
result. A nonzero elevated exit has no proof route.

## Exclusions

This does not add a default installer command, a UAC launcher, a UI dialog, a
shortcut, an uninstall entry, automatic repair, rollback, data migration,
restart, notification, background service, or application protocol capability.
Those each need their own host-owned boundary.

See [Windows installer](WINDOWS_INSTALLER.md), [update acceptance](UPDATE_ACCEPTANCE.md),
and Decision 0177.
