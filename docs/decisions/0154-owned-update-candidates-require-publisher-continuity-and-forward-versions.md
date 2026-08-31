# Decision 0154: Owned update candidates require publisher continuity and forward versions

**Status:** Accepted

**Date:** 2026-08-31

## Context

An update changes executable code selected by the machine policy record. A
valid signed candidate alone is insufficient: accepting a differently signed
application would break the installed publisher binding, while accepting an
equal or older release would permit rollback or replay.

The product owner has deliberately not selected a production certificate
custody model or update delivery source. The first owned updater slice must
therefore establish local acceptance rules without inventing a URL, network
client, service, command, or background schedule.

## Decision

The update preflight verifies the current signed resource-bearing installer,
then reads the fixed machine record for its embedded application identity. It
verifies the selected installed executable with Windows Authenticode and
requires that accepted signer to match the installed record and the candidate
manifest publisher.

The selected package root must end in the exact canonical release-directory
name `major.minor.patch`. The candidate's manifest version must be strictly
greater than that parsed installed version. The preflight returns only an
opaque candidate result; it accepts no application identity, root, executable,
publisher, version, registry location, URL, channel, or policy input.

## Consequences

- A valid candidate cannot silently change publisher identity or replace an
  equal or earlier selected Anodrel release.
- Alternate numeric spellings such as `01.2.3` are not version identities.
- The gate is read-only and creates no update route by itself. The later staged
  executable signer gate remains mandatory before publication.
- Key rotation, delivery metadata, download transport, user consent,
  transaction composition, and rollback/recovery stay explicit future gates.

## Revisit conditions

Revisit for an approved publisher-key rotation policy, a signed update-index
format, a selected delivery origin, or a Windows package distribution model.
