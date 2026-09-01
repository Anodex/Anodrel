# Decision 0159: Owned updates retain one prior fixed policy record

**Status:** Accepted

**Date:** 2026-08-31

## Context

An Anodrel update publishes a new complete version by replacing the one fixed
machine `record` value. Versioned directories retain the earlier files, but
directory discovery alone cannot safely reconstruct that release's prior
capability policy or prove that it is the intended rollback target.

## Decision

The update-only policy-publication boundary reads the existing fixed `record`
value and writes its exact bounded `REG_SZ` text to a second fixed value named
`previous` in the same 64-bit machine key. It then writes the already validated
new `record`. No initial installation, application, command-line argument, or
general registry API can select either value name.

The current host continues to read only `record`; `previous` is private
installer recovery material. If writing `previous` fails, the current record is
unchanged. If a later new-record write fails, the prior record remains selected
and the complete promoted directory remains unselected for recovery.

## Consequences

- One policy-complete prior release is retained for a future rollback path.
- Updates do not infer a rollback target by enumerating directories or copying
  permissions from the currently selected version.
- This is not a rollback command, a multi-version history, automatic recovery,
  delivery mechanism, or publisher-key rotation policy.

## Revisit conditions

Revisit for a policy-backed rollback command, a larger bounded history, a
transactional policy service, publisher-key rotation, or an enterprise updater.
