# Decision 0010: Start application hosting with a verified owned text package

**Status:** Accepted

**Date:** 2026-07-31

## Context

The Windows host can create an internal diagnostics window and can privately bootstrap a development child process. Neither path establishes an application identity or safely loads application-controlled content. Allowing arbitrary files, scripts, browser content, navigation, or a general bridge before that boundary exists would make identity and native authority implicit.

The project intentionally does not ship a browser engine, webview framework, or third-party UI runtime. A first step must therefore be small enough to own, audit, and test with direct Win32 APIs.

## Decision

Anodrel defines application package manifest 1.0 in `docs/APPLICATIONS.md`. A package contains a strict manifest and one digest-verified `anodrel.text.v1` document. The host canonicalizes paths, enforces package containment, validates bounded plain text, and draws that text in an Anodrel-owned Win32 window.

The validated `applicationId` is the identity associated with this content surface. No request payload, rendered text, command-line child, or pipe client may supply that identity to the host. This increment creates no executable launch authority, native bridge, script execution, navigation, resource loading, application permissions, or publisher-signature claim.

## Consequences

Positive:

- application content has a concrete host-validated identity before a privileged capability or a general renderer is introduced;
- package traversal, oversized content, malformed manifests, and tampering can be tested without a browser or framework runtime;
- the text host proves a direct owned content boundary while keeping Win32 UI code and portable package validation in separate modules.

Tradeoffs:

- the first surface is deliberately not a productive application UI;
- a raw content digest detects accidental or untrusted modification but does not authenticate a publisher who can replace the whole unpackaged package;
- executable trust, signing, updates, and a secure UI runtime still need separate decisions.

## Revisit conditions

Revisit this decision when Anodrel defines signed package distribution, verified executable launch, or an additional content format. Any such format must retain explicit application identity, containment, validation, and resource limits before it can receive a native bridge or capability.
