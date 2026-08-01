# Decision 0025: Start native UI with a constrained declarative foundation

**Status:** Accepted

**Date:** 2026-08-01

## Context

Anodrel deliberately does not ship Chromium, Node.js, a webview framework, or a
third-party UI runtime. The verified plain-text package proves a content
boundary but is not sufficient for a productive application. Replacing a
browser with a full UI framework in one change would create a large, implicit,
and difficult-to-audit surface.

## Decision

Anodrel owns a portable `anodrel-ui` crate with a constrained in-memory view
tree: stacks, text, and semantic actions. It validates bounded plain data,
performs deterministic host-measured layout and clipping, and returns only a
semantic action event from hit testing. The exact model, limits, and omissions
are documented in `docs/UI.md`.

The crate is not an application package format, protocol message, renderer,
window API, script engine, accessibility system, or native bridge. It has no
operating-system or third-party dependency. Host renderers and future
authenticated application sessions remain separate adapters above it.

## Consequences

- Anodrel gains an owned, testable UI contract without committing to a browser
  engine or a framework runtime;
- host text shaping remains an explicit platform seam;
- an action cannot become native authority merely by being visible or clicked;
  and
- a useful interactive application surface still requires rendering, input,
  accessibility, session, and capability decisions.

## Revisit conditions

Revisit before accepting an untrusted UI document, adding another node type,
exposing a public action/session API, or giving an action any native effect.
