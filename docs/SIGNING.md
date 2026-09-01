# Windows executable-signature foundation

**Status:** Windows verification is implemented. The owned release signing
boundary is documented and selects no production identity by itself. This
document does not authorize application process launch.

## Purpose

Anodrel needs a way to identify the publisher of a Windows executable before a
future host may launch it. The first adapter, an
anodrel-windows-signature crate, calls Windows Authenticode and certificate
APIs directly. It introduces no third-party runtime, command shell, scripting
host, or browser component.

The adapter accepts a canonical path held by host policy and returns only the
SHA-256 fingerprint of the leaf signing certificate when Windows accepts the
embedded Authenticode signature. It does not return native status codes,
certificate subjects, raw paths, signature blobs, or trust-provider diagnostics
to a renderer or application.

## Current contract

The public operation is equivalent to:

~~~text
verify_embedded_signature(canonical_executable_path)
    -> trusted leaf certificate SHA-256 fingerprint
    | safe verification failure
~~~

## Owned release-signing contract

`anodrel-release-sign sign <unsigned-release-image> <certificate-sha256>
<new-signed-image>` is a release-operator boundary, not a platform service or
application capability. It first verifies that its absolute input carries an
exact Anodrel release manifest and bundle whose publisher value equals the
supplied fingerprint. It copies that input to one absent absolute output, opens
only the current user's Windows `MY` certificate store, and selects only that
exact lowercase SHA-256 fingerprint. It asks Windows `SignerSignEx` for a SHA-256 Authenticode
signature with no timestamp or network endpoint, then requires Windows trust
to accept the new image and report the same leaf fingerprint.

The signer accepts an image no larger than **576 MiB**: Anodrel's fixed 512 MiB
release payload limit plus a bounded 64 MiB PE envelope. It streams the copy
into a create-new output file, synchronizes that file before signing, and
removes only that fresh output on any later failure.

It does not modify the input, choose by certificate subject, offer a picker,
fall back to another store or certificate, create a certificate, import a key,
install machine trust, timestamp, download, install, or launch. An error
removes only the fresh output created by that invocation. A private key,
certificate authority, trust distribution, timestamp service, and renewal plan
remain operator-owned production decisions.

The Windows adapter:

1. asks WinVerifyTrust to evaluate the embedded Authenticode signature without
   showing UI;
2. retrieves the leaf certificate only from the successful trust-provider
   state;
3. reads the certificate's SHA-256 fingerprint through CryptoAPI; and
4. closes the provider state before returning.

The result is a fixed 32-byte value. A caller that needs display text must
obtain it through a separately reviewed package or installation policy; subject
names are not a stable authorization primitive.

## What this proves, and what it does not

| Condition | Current adapter |
| --- | --- |
| Embedded executable signature is accepted by Windows policy | Yes |
| Leaf signer certificate fingerprint is available for comparison | Yes |
| An executable is contained below its package root | No — future package loader responsibility |
| Executable bytes match a declared digest | No — future package loader responsibility |
| The signer is authorized for a specific application identity | No — future installed publisher-policy responsibility |
| A package manifest itself is trusted | No |
| An executable may be launched | No |
| A child receives a bootstrap invitation | No |

An unpackaged directory can be replaced as a whole. A manifest inside that same
directory cannot serve as the trust anchor for its own signer fingerprint.
Consequently, a future launch policy must obtain the allowed publisher
fingerprint and application identity from an installation record or
operating-system package identity outside the mutable application directory.
`docs/LAUNCH.md` now defines the strict installation-record contract. The
query-only Windows policy store and host-only launch service consume that
record, while provisioning a signed application record remains later work.

## Launch gate

The host-only launch service performs all of the following before CreateProcessW:

1. resolves and contains the executable below the approved package root;
2. checks its declared SHA-256 digest;
3. calls this adapter and matches the leaf fingerprint to the external
   installed application record;
4. binds that publisher policy to the validated application ID;
5. creates and tracks the child without shell interpretation; and
6. uses the existing one-use private bootstrap only after identity validation.

Each failure is fail-closed, safe to render, and leaves no child process. The
service locks the executable against write, delete, and rename handles while it
checks the digest, invokes Windows trust policy, and creates the child. The
work must run off the Win32 UI thread because Windows trust policy can consult
certificate and revocation state. A provisioned signed sample and host UI
integration remain required before any product launch exists. The Startup Lab's
one launch control is the **Development Fixture** tile, which activates a
development fixture and is never presented as a product launch; production
signing identity and packaging are deliberately deferred in `ROADMAP.md`.

## Logging and privacy

The typed host log must not record paths, native trust errors, certificate
subjects, fingerprints, executable arguments, or invitations. Future UI may
show only a host-defined readiness state after an installed publisher policy is
implemented.

## Verification

The adapter has pure tests for its path and fingerprint handling. Its
Windows-only integration test, intentionally opt-in, receives an operator
selected embedded-signed executable and confirms that an unsigned test binary
is rejected. Manual native verification records only pass or fail; it does not
capture or display signer data.

Decision 0017 records why this adapter remains a verification primitive rather
than a launch permission.
