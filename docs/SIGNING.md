# Windows executable-signature foundation

**Status:** Windows foundation. This document does not authorize application
process launch.

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
fingerprint and application identity from a signed installation record or
operating-system package identity outside the mutable application directory.

## Future launch gate

The Launch Sample action stays planned until one host-controlled operation
performs all of the following before CreateProcessW:

1. resolves and contains the executable below the approved package root;
2. checks its declared SHA-256 digest;
3. calls this adapter and matches the leaf fingerprint to a publisher policy
   held outside the package;
4. binds that publisher policy to the validated application ID;
5. creates and tracks the child without shell interpretation; and
6. uses the existing one-use private bootstrap only after identity validation.

Each failure must be fail-closed, safe to render, and leave no child process.
The verification work must run off the Win32 UI thread, because Windows trust
policy can consult certificate and revocation state.

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
