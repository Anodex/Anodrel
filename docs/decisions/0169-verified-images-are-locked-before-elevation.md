# Decision 0169: Verified images are locked before elevation

**Status:** Accepted

**Date:** 2026-09-01

## Context

The updater can retrieve an installer image that matches a signed catalogue,
but that user-writable cached file must not become an administrator process
based only on a previous hash check. A separate verify-then-launch sequence
would leave a replacement race between verification and the UAC request.

## Decision

Accept a downloaded installer only through a native image gate that maps the
exact file as a Windows resource image with exclusive write protection. While
that mapping remains alive, Windows Authenticode must accept the file; its two
fixed release resources must form a valid complete release; and its identity,
version, and publisher must exactly match the already CMS-verified catalogue.

The direct elevation adapter consumes only this opaque locked result. It calls
Windows with the explicit `runas` verb and the fixed `update` command, retains
the resulting process handle, and exposes no file, parameter, working-directory,
or shell verb selection. The elevated installer must independently re-run its
normal signature, release, policy, publisher, and update transaction gates.

## Consequences

- A digest-checked cache path alone cannot reach UAC.
- Image inspection does not execute the candidate or rely on a third-party
  installer framework.
- A successful UAC launch is not treated as installation proof.
- Cache-root selection, user-visible consent, process waiting, recovery, and
  restart remain separate native ownership boundaries.

## Revisit conditions

Revisit for a dedicated privileged broker, a protected machine cache, a signed
package format other than a PE installer, multi-user update policy, a restart
coordinator, or another operating system.
