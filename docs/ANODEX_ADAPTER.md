# Anodex Adapter Plan

**Status:** First migration boundary implemented and contract-tested; no
Anodex runtime is hosted by Anodrel yet.

## Purpose

Anodex and Anodrel remain separate repositories. The migration must replace
desktop services behind an adapter while Anodex's Electron implementation
continues to work. It must not copy Anodex source into Anodrel, add Anodex
concepts to the platform core, or claim that a development diagnostic is a
product migration.

## First boundary: title-bar window state

The current Anodex title bar uses four Electron-backed behaviours:

| Anodex need | Existing Anodrel boundary | Migration status |
| --- | --- | --- |
| minimise its own window | `window.state.set` with `minimized` | Implemented |
| choose maximise or restore | `window.state.set` with a closed value | Implemented |
| close the current application | `session.close` | Semantically different; requires lifecycle review |
| show the correct initial maximise/restore glyph | `window.state.get` | Implemented in `@anodrel/anodex-adapter` |
| react to an ordinary Windows maximise/restore change | `window.state.changes.read` | Specified; no hidden polling or callback shim |

Anodrel intentionally has no native equivalent of Electron's maximise-change
event. The first adapter must use one initial state snapshot and refresh after
its own request; it must not invent a background event channel. Anodrel also
does not yet host Anodex's React renderer, workspace, model runtime, or its
domain services.

## Migration sequence

1. **Completed:** implement and verify the separate `window.state.read` protocol capability.
2. **Completed:** add a small Anodex-specific adapter package outside the platform core. It
   maps only the closed title-bar service contract and has no Electron import.
3. Add the corresponding Electron adapter in Anodex, preserving its current
   renderer-facing contract while both adapters are verified against the same
   adapter tests.
4. Compare behaviour in an explicit development route before moving another
   desktop service. A failure must leave the Electron adapter selectable.

## Non-goals

- importing, moving, or compiling Anodex source from this repository;
- a webview, browser engine, Node.js runtime, or React host inside Anodrel;
- a general Electron compatibility layer;
- native window handles, geometry, shell events, global shortcuts, or process
  management exposed to Anodex code; and
- switching Anodex production builds before its full UI and recovery path are
  proven.

## Evidence standard

Every later adapter slice needs a documented mapping, contract tests shared by
both adapters, a native-host verification, a comparison against the Electron
behaviour, and a rollback path. See `docs/WINDOW_STATE_OBSERVATION.md`,
Decision 0117, and Phase 4 in `docs/roadmap/FUTURE.md`.
