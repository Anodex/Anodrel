# Decision 0198: Apps & features removal uses fixed native consent

**Status:** Accepted

**Date:** 2026-09-04

## Context

The Apps & features registry entry can point at only the selected signed
uninstaller image. Its internal `uninstall` command correctly requires an
already elevated token, but Windows starts an uninstall command from a normal
user context. Exposing that command directly would leave a person with a
failure message instead of a removal flow.

Letting the registry choose an executable, argument, verb, window, policy, or
elevation target would turn a display integration into general process
authority.

## Decision

Add one fixed `remove` route to the installed signed uninstaller. It first
performs the existing selected-image, version, publisher, and policy proof.
Only then does it show one direct native confirmation with an explicit remove
or cancel choice. Decline stops with no UAC request or policy change.

Approval permits only one `ShellExecuteExW` call with the `runas` verb, the
current verified uninstaller image, and the fixed internal `uninstall`
argument. The elevated child repeats the existing preflight before it changes
policy. A zero exit is followed by a fresh fixed-policy absence proof; it is
not presented as proof merely because a process returned successfully.

The Apps & features registry writer will use the fixed `remove` route. Named
`install`, `update`, `rollback`, and `uninstall` remain explicit advanced
commands requiring an already elevated shell.

## Consequences

- Apps & features can offer direct native consent and UAC without a shell,
  script, framework installer, application-controlled argument, or background
  service.
- The interactive route has no silent removal mode, repair, target selection,
  progress telemetry, restart request, or application protocol surface.
- The current image remains the only executable passed to Windows, and the
  elevated child retains the same version-bound removal constraints.

## Revisit conditions

Revisit for an approved repair flow, a managed silent deployment interface,
packaged Windows distribution, per-user installation, product localization,
multi-application operations, or another operating system.
