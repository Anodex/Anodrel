# Decision 0020: Launch only a locked and revalidated Windows executable

**Status:** Accepted

**Date:** 2026-07-31

## Context

The installed-record parser, machine policy store, Authenticode adapter, and
private bootstrap launcher are deliberately separate. Joining them by verifying
a path and later passing that same mutable path to `CreateProcessW` leaves a
write, delete, or rename race. It would also make it easy to accidentally pass
application-controlled arguments or let child lifetime escape the host.

## Decision

Add `anodrel-windows-launch`, a host-only Windows service. It reads a
host-selected installed record, locks its canonical executable with direct
`CreateFileW` read access and `FILE_SHARE_READ` only, checks containment and
the record digest through that lock, performs Authenticode verification while
the lock is held, and compares the accepted leaf certificate fingerprint to the
record's approved value.

Only after all checks pass does it call the existing bootstrap launcher with
the exact `.exe` and no child arguments. The launcher delivers the one-use
invitation after process creation and terminates the child if delivery fails.
The service returns a tracked child object; dropping it terminates the child,
and the host may wait for its exit code or terminate it explicitly during
shutdown.

The service receives its application ID and invitation from host code. It does
not select policy, create a pipe, accept application data, inspect process
output, restart a child, log paths or certificate information, or expose any
public protocol capability.

## Consequences

Positive:

- the process image cannot be replaced between verification and creation;
- executable, signer, policy, bootstrap, and lifecycle checks remain explicit
  modules with narrow interfaces;
- no shell, argument forwarding, or inherited ambient application authority is
  introduced;
- child lifetime has a concrete shutdown owner.

Tradeoffs:

- the lock can transiently prevent an updater from replacing the executable;
- Authenticode evaluation and process creation remain blocking worker work;
- an opt-in signed and machine-provisioned test fixture is required for a full
  launch integration test.

## Revisit conditions

Revisit when Windows package identity supplies an immutable process image,
when update coordination needs a separate lock protocol, when a child requires
documented non-secret arguments, or when a brokered process model replaces
direct child creation.
