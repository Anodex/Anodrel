# Future phases and deliberate deferrals

## Phase 4 — Anodex adapter

Status: **First migration boundary implemented; broader migration planned**

- Connect Anodex through the public Anodrel interfaces.
- **First boundary implemented:** Anodex's title bar can use the separate
  pull-only `window.state.read` capability through `@anodrel/anodex-adapter`.
  It does not claim that Anodex's React runtime can run on Anodrel.
- Keep Anodex's existing Electron adapter working during migration.
- Move platform-specific operations behind the new adapter.
- Compare behavior and performance between the old and new hosts.
- Switch Anodex only after feature parity and recovery procedures are proven.

Acceptance gate: Anodex can run on Anodrel without importing Electron APIs
from its core application logic.

## Phase 5 — Additional applications and platforms

Status: **Second-sample foundation implemented; platform expansion planned**

- Add a second sample application. **Completed for the current static package
  boundary:** `apps/compass` carries a distinct application identity and its
  own digest-verified text surface, validated in the shipped-package test. It
  adds no executable runtime, capability, native bridge, or Anodex dependency.
- Add macOS and Linux host adapters as resources allow.
- Stabilize the protocol and publish SDK documentation.
- Define a long-term release and support policy.

## Explicitly deferred

- **Production signing identity, packaging, installation, and updates.**
  Deferred by decision, not by oversight. Until it is made, the platform has no
  production application identity, so the only thing it can provision is the
  development fixture of Decision 0061, and toast notifications stay out of
  reach because they need an Application User Model ID this platform cannot
  honestly claim. Nothing built so far may be presented as production-ready.
- A full native UI toolkit beyond the constrained foundation in Decision 0025.
- Custom browser engine.
- Custom operating system.
- Full Anodex rewrite before the platform contracts are proven.
