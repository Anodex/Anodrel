# Decision 0172: Update success requires postcondition proof

**Status:** Accepted

**Date:** 2026-09-01

## Context

Windows accepting UAC and returning a process handle proves only that the
handoff started. Even an installer exit code of zero does not independently
show what the machine policy now selects. Treating either as update success
would overstate an unverified outcome.

## Decision

After a fixed elevated installer process exits with code zero, re-read only the
machine record for the candidate's already verified application identity.
Require the record to validate, its selected executable to pass Windows
Authenticode, its signer to match the record and candidate release, and its
canonical selected version directory to equal the candidate version. Return an
opaque proof only after every condition holds.

Do not attempt this check after a nonzero installer exit. The proof does not
restart a process, mutate policy, remove a file, report progress, expose facts,
or turn exit status into a public application result.

## Consequences

- The native updater has a precise postcondition instead of assuming process
  termination implies installation.
- Failed or unknown installer outcomes remain unaccepted and non-destructive.
- A real signed end-to-end acceptance remains necessary to validate Windows,
  UAC, installer behavior, and certificate configuration together.

## Revisit conditions

Revisit for multiple selected records, application data migration checks,
restart coordination, rollback UX, a privileged broker, another platform, or
an application-visible update status protocol.
