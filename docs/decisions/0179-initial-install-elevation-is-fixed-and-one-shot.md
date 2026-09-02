# Decision 0179: Initial-install elevation is fixed and one-shot

**Status:** Accepted

**Date:** 2026-09-02

## Context

After a person approves a signed initial installation, the process must cross
the Windows administrator boundary without accepting an executable path,
argument, or generic elevation request. Reusing a broad shell helper would add
authority that the owned installer does not need. A zero elevated-process exit
is also insufficient evidence that machine policy selected the release.

## Decision

Accept only `ApprovedInitialInstall` from the native consent boundary. Obtain
the current installer image internally, require an absolute path, and call
direct `ShellExecuteExW` with only the literal `runas` verb and `install`
argument. Retain exactly the returned process handle for later wait. On a zero
exit, allow only the existing opaque initial-install postcondition proof; do
not call an exit success installation success.

The handoff accepts no image, argument, working directory, shell verb, owner
window, package, policy, certificate, endpoint, command output, or restart
input. It does not expose a generic elevation API.

## Consequences

- UAC cancellation, launch failure, missing process handles, and wait failures
  have safe, path-free operator outcomes.
- The elevated child repeats its own signature and installation gates before it
  can change policy.
- A later interactive installer coordinator can compose consent, handoff, and
  postcondition proof without inventing an administrative shell surface.

## Revisit conditions

Revisit for a dedicated privileged broker, a native installer window,
installation progress, managed deployment, alternate scope, restart
coordination, a signed positive fixture run, or another operating system.
