# Decision 0022: Use a narrow current-user Windows Credential Manager store

**Status:** Accepted

**Date:** 2026-07-31

## Context

Anodrel needs operating-system-backed storage for small application secrets.
Putting tokens in a package, environment variable, command line, shared data
directory, or general diagnostic log would weaken the host boundary. A generic
credential interface, however, could allow an application to read unrelated
Windows credentials or leak secrets before session permissions are designed.

## Decision

Add a portable `anodrel-credentials` crate with a validated credential name,
an opaque bounded secret value, and a target derived only from a validated
application identity. Add `anodrel-windows-credentials`, which uses direct
Advapi32 Credential Manager APIs for `CRED_TYPE_GENERIC` credentials persisted
for the current user on the local machine.

The target is fixed to `Anodrel/v1/<applicationId>/<credentialName>`. Names
cannot contain separators or Windows credential prefixes. Reads, writes, and
deletes are exact-target operations; enumeration, prompts, sharing, export,
attributes, user names, roaming, and arbitrary targets are not implemented.
Secrets are limited to 2,048 bytes, redacted in debug output, and cleared when
the Anodrel secret value or a Windows-returned buffer is released.

The service is host-only. It accepts no application protocol request and must
not be attached to a rendered client until explicit credential capabilities,
session binding, revocation, cancellation, and compatibility tests are defined.

## Consequences

Positive:

- secrets use Windows' current-user credential store rather than an Anodrel
  file format or a third-party runtime;
- the target namespace and validated application identity prevent accidental
  cross-application target selection;
- secret handling and raw Windows memory ownership are isolated from protocol,
  UI, diagnostics, and future storage services.

Tradeoffs:

- this initial adapter cannot serve a machine service, another Windows user,
  or a roaming-profile policy;
- a token larger than 2,048 bytes needs a separately designed encrypted-storage
  format rather than silently expanding this contract;
- an integration test briefly creates and then removes one scoped test
  credential from the current user's store.

## Revisit conditions

Revisit for a public credential capability, a cross-platform keychain mapping,
hardware-backed keys, user-approved sharing, service-account credentials,
roaming policy, secret rotation, or data migration from this namespace.
