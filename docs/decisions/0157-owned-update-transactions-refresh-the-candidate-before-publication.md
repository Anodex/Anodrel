# Decision 0157: Owned update transactions refresh the candidate before publication

**Status:** Accepted

**Date:** 2026-08-31

## Context

The update preflight establishes that the current signed installer release has
the selected application's publisher and a strictly newer version. A later
filesystem and registry transaction must not blindly rely on that earlier read:
the candidate image may be replaced before staging begins, even if Windows will
accept the replacement signature.

## Decision

The no-argument update transaction first obtains the opaque preflight result,
then activates the current signed installer release again before it selects a
machine root. The refreshed manifest must retain the preflight candidate's
application identity, package version, and installed publisher fingerprint.
Only that refreshed verified release can enter private preparation. The existing
staged executable signer, no-overwrite promotion, and fixed policy publication
boundaries complete the transaction.

## Consequences

- A changed current installer cannot convert a prior candidate decision into a
  different release, even when both installer images are accepted by Windows.
- Versioned directories let an already running previous release remain intact
  while the machine policy begins selecting the complete new version.
- No downloader, background updater, URL, channel, user-data operation, or
  publisher-key rotation policy is introduced.
- The initial-install transaction remains separate and rejects any existing
  selected policy, so it cannot bypass forward-version checks.

## Revisit conditions

Revisit for an approved signed update index, publisher-key rotation, a rollback
policy record, an update delivery origin, or a background update service.
