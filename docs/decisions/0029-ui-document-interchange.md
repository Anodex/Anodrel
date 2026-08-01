# Decision 0029: Native UI document interchange is strict and capability-free

**Status:** Accepted

**Date:** 2026-08-01

## Context

The portable UI tree is useful only to a host-owned caller until an application
can describe one through a stable interchange boundary. Sending arbitrary
renderer data or allowing a generic JSON object would recreate the hidden,
unbounded coupling that the constrained model deliberately avoids. Treating a
document as a host operation would also risk confusing visual data with native
authority.

## Decision

Anodrel defines `anodrel.ui.document.v1` as an exact, bounded UTF-8 JSON
representation of the existing UI tree. A dedicated `anodrel-ui-document`
crate depends only on Anodrel's strict JSON codec and the in-memory UI model.
It validates a complete document in memory and provides deterministic encoding
of an already-valid document.

The format has an exact version marker, rejects unknown and missing fields, and
applies both a 64 KiB encoded limit and the UI model's existing structural
limits. It carries neither capability declarations nor callbacks, renderer
objects, native handles, file paths, URLs, scripts, package locations, or
window instructions.

The Windows UI Lab may decode one compile-time fixture held in host source to
exercise the full schema-to-renderer path. It does not read a document from a
package, command line, file, pipe, protocol operation, or application session.
No native host accepts an externally supplied format in this decision. Such a
consumer requires a separate lifecycle, queue, permission, and threat-model
decision.

## Consequences

- future SDKs and package formats have a small, documented source of truth for
  portable UI data;
- compatibility tests can reject an accidental schema change before a host
  interprets it; and
- untrusted visual data remains separate from operating-system authority.

## Revisit conditions

Revisit before a host renders externally supplied documents, an application
session transports documents, incremental updates are introduced, a document
can refer to an asset, or an action event reaches application code.
