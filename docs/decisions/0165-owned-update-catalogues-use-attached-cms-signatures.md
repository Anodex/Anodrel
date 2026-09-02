# Decision 0165: Owned update catalogues use attached CMS signatures

**Status:** Accepted

**Date:** 2026-08-31

## Context

The strict update catalogue names an exact candidate image but is not a trust
anchor by itself. Keeping it beside a signed installer would not help a later
network client, while using an external signing service or updater framework
would weaken Anodrel's direct ownership boundary.

Windows CryptoAPI can create and verify an attached CMS message using an exact
certificate in the current user's `MY` store. The existing installed publisher
fingerprint is already the narrow authorization value for signed release images.

## Decision

Sign the strict catalogue's exact UTF-8 bytes as one attached CMS message with
one explicitly selected SHA-256 certificate fingerprint. Include that signer
certificate in the message. A verification adapter requires exactly one valid
CMS signer and compares its SHA-256 certificate fingerprint to the current
installed publisher before it returns the decoded bytes for catalogue parsing.

The signed envelope is bounded to 128 KiB and its decoded catalogue to 16 KiB.
The adapter uses `CryptSignMessage`, `CryptGetMessageSignerCount`,
`CryptVerifyMessageSignature`, and `CertGetCertificateContextProperty` directly.
It suppresses key-provider UI, does not download content, and does not change
certificate stores or trust.

CMS verification proves possession of the pinned publisher's private key; it
does not itself establish certificate-chain trust, timestamp validity, or
installer acceptance. The downloaded installer must still pass the existing
Windows Authenticode and installer update gates.

## Consequences

- The delivery metadata has a small signed, bounded format without a feed
  framework, JSON signature dependency, or sidecar signature ambiguity.
- A stolen publisher private key is still security-critical, just as it is for
  Authenticode release images; no catalogue signature can mitigate that alone.
- Certificate expiry, revocation, rotation, trusted timestamping, and update
  endpoint retrieval stay distinct product decisions.

## Revisit conditions

Revisit for certificate rotation, a hardware-backed or remote key provider,
multiple signers, a trusted timestamp policy, a non-Windows release route, or
an operating-system package updater.
