# Decision 0148: Publish only a promoted validated record

**Status:** Accepted

**Date:** 2026-08-31

## Context

The Windows host reads exactly one machine policy value at
`HKEY_LOCAL_MACHINE\Software\Anodrel\Applications\<applicationId>\record`.
After a release is verified and promoted, that value is the only authority that
can make the new directory launchable. A general registry API or command-line
record input would let unverified policy bypass the installer checks.

## Decision

Add a direct Advapi32 publication boundary to the owned installer. It accepts
only the opaque result of verified same-volume promotion, derives the fixed
64-bit machine key from its validated identity, and writes one UTF-16 `REG_SZ`
value named `record`. The record was already rendered and validated against the
promoted package before this call.

The boundary does not read or enumerate registry keys, choose a hive or key,
delete a version, install trust, launch a process, or accept record text from a
command line. If publication fails, the prior record remains selected and the
complete new directory stays unselected for later recovery. After publication,
the host's normal locked launch path revalidates the selected executable.

## Consequences

- Registry authority remains one fixed value, not a general installer feature.
- A version directory is selected only after all filesystem and signer gates.
- A signed positive end-to-end registry test still requires the production or
  development signing fixture and elevation.

## Revisit conditions

Revisit for a Windows package identity, enterprise policy broker, or a
transactional registry/policy service.
