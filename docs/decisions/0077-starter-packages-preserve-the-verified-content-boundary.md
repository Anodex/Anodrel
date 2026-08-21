# Decision 0077: Starter packages preserve the verified content boundary

**Status:** Accepted

**Date:** 2026-08-21

## Context

Phase 3 needs a person to be able to create and run a small project without
reading host internals. The current public application package is deliberately
smaller than a general application runtime: it contains one manifest and one
digest-verified plain-text document. It does not execute application code.

A starter generator that creates an executable, embeds a private bootstrap
client, writes a policy record, or silently overwrites a directory would claim
product capabilities that the current package boundary does not provide.

## Decision

The first starter tool creates only an `anodrel.text.v1` package in a new,
operator-named directory. It validates the application ID, display name, and
plain text against the published package limits before writing anything. It
normalises text line endings to LF, writes UTF-8 without a byte-order mark,
computes the SHA-256 digest over those exact content bytes, and writes the
strict version 1.0 manifest that names that digest. It also writes the small
package-local Git attribute rule that prevents a future checkout from applying
line-ending conversion to the digest-verified text.

The tool refuses an existing destination. It does not install packages, invoke
the host, start a process, change machine state, create an installed-record,
grant a capability, or sign content. The operator explicitly starts the
resulting package through the documented Windows-host command; the host still
independently validates containment, manifest shape, text constraints, and the
digest before it creates a window.

## Consequences

- A new user has one small, runnable project at the boundary Anodrel actually
  supports today.
- The generated package remains useful as a content-integrity and native-host
  smoke test.
- The tool cannot be mistaken for a production packager, application SDK
  scaffold, executable launcher, or updater.

## Revisit conditions

Revisit when a stable executable application contract, published SDK packages,
production signing identity, packaging, installation, or updates exist. Each
would need its own template, threat-model review, and documentation rather than
being folded into this content-package tool.
