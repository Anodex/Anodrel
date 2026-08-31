# Decision 0140: Own the Windows installer and keep its release envelope signed

**Status:** Accepted

**Date:** 2026-08-30

## Context

Windows product launch already validates an externally selected machine record,
the contained executable digest, and its Authenticode publisher fingerprint.
What remains is a production mechanism to install that record and its package
without adding an installer framework, a scripting host, or a mutable package
directory as a trust source.

An installer that accepts an arbitrary directory, downloaded archive, registry
path, or policy value would reopen the authority boundary that the installed
record deliberately closes. A separate unsigned release manifest would do the
same: an attacker could pair a legitimate installer with altered application
facts.

## Decision

Build a first-party `anodrel-windows-installer` over direct Windows APIs. Its
release manifest and payload are embedded in the same Authenticode-signed
installer executable. Version 1 is machine-scoped and needs elevation; it
writes only the existing installed-record value beneath the fixed Anodrel
machine-policy location.

The embedded `anodrel.release.v1` manifest binds one application identity,
semantic package version, executable digest, publisher fingerprint, fixed
capability policy, exact network origins, and payload digest. The installer
accepts no arbitrary package root, registry key, value name, capability, or
network location.

Before it publishes a record, the installer verifies its own signature,
validates the embedded release manifest and payload, extracts only into a new
host-selected version directory, verifies the staged package and executable,
and validates the record it intends to write through the host's existing
installed-record validator. Initially, the installer and installed executable
must have the same accepted leaf fingerprint. Publisher-key rotation is a
separate future update decision.

## Consequences

Positive:

- Anodrel owns the installer, bundle parser, staging, registry provisioning,
  uninstallation, and recovery logic instead of taking a runtime dependency.
- Authenticode covers both installer code and its release authority data.
- A host can see either the old complete record or the new complete record,
  never a package-selected policy value.
- The design directly reuses the locked-launch and installed-record boundaries.

Tradeoffs:

- A publicly trusted release still requires an operator-selected code-signing
  certificate and private-key custody plan.
- Machine installation requires elevation and must have explicit recovery and
  uninstall behaviour.
- Version 1 intentionally omits download, automatic update, key rotation,
  shortcuts, and package-framework integration until each has its own contract.

## Revisit conditions

Revisit when Anodrel chooses an operating-system package identity, defines
publisher-key rotation, adds an enterprise deployment route, or needs a
separate user-scoped development installation model.
