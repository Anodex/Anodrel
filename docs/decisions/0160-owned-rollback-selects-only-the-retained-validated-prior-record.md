# Decision 0160: Owned rollback selects only the retained validated prior record

**Status:** Accepted

**Date:** 2026-08-31

## Context

An update retains one private policy-complete prior record. A rollback must not
turn that retained text into a generic downgrade switch: both current and prior
packages can be mutable filesystem inputs, and the current installer must not
select an arbitrary path or version.

## Decision

Rollback starts from the current signed embedded installer release. It reads
only the fixed `record` and fixed `previous` values for that release identity,
validates both complete packages, and checks both executable Authenticode
signers against their own records and the installer publisher. Both canonical
package roots must be direct version children of the existing owned machine
root; the prior version must be strictly lower than the selected version.

Only the resulting opaque target can copy fixed `previous` policy text to fixed
`record`. It accepts no identity, version, path, registry value, publisher,
package, URL, channel, or policy input. It leaves both package directories and
the retained value intact, giving the next approved update one bounded slot to
replace.

## Consequences

- Rollback cannot be used for same-version replay or directory discovery.
- The previous release's own capability policy and executable binding return
  together; no permissions are synthesized from the current release.
- This does not stop a running application, delete new files, create a history,
  download code, or provide automatic rollback.

## Revisit conditions

Revisit for multi-level history, automatic health-triggered rollback, a process
restart policy, publisher-key rotation, or transactional machine policy.
