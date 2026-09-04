# Windows Apps & features removal flow

**Status:** Contract accepted; implementation pending.

## Purpose

An Apps & features entry must work from a normal Windows user context while
keeping removal authority fixed. It never invokes a downloaded installer,
application child, shell command, user-supplied argument, or application
protocol request.

## Fixed sequence

~~~text
selected signed uninstaller image
    -> selected policy, version, publisher, and image proof
    -> one native Remove / Cancel confirmation
    -> one fixed UAC handoff to the same image's internal uninstall command
    -> elevated removal repeats proof
    -> fixed policy-absence proof
~~~

The uninstaller image and the `remove` and `uninstall` route names are Anodrel
constants. The registry entry contains only the quoted selected image path and
the fixed `remove` route. It does not quote or pass product display text,
application identity, package path, registry key, version, publisher, or
network location.

## Boundaries

The first proof does not modify policy, create a process, elevate, display
progress, delete files, or retain a preference. Native cancellation does not
start UAC. The UAC child repeats preflight rather than trusting a normal-user
process. A successful child exit requires the final policy proof, which does
not claim that restart-delayed self-cleanup has completed.

## Exclusions

This flow does not add silent uninstall, repair, custom install UI, arbitrary
arguments, restart control, data deletion, updater service, telemetry,
Application User Model ID, or application control over registration.

See [Apps & features](APPS_AND_FEATURES.md), [Windows installer](WINDOWS_INSTALLER.md),
and Decision 0198.
