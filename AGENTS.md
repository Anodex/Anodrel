# Anodrel Agent Instructions

These instructions apply to the entire Anodrel repository.

## Before changing code

1. Read README.md.
2. Read ROADMAP.md.
3. Read docs/ARCHITECTURE.md.
4. Check docs/decisions/ for decisions related to the area being changed.
5. Inspect the current Git status and preserve unrelated work.

## Project boundaries

- Anodrel is independent from Anodex and must remain in its own repository.
- Do not copy Anodex source files into Anodrel as a shortcut.
- Do not add Anodrel files to the Anodex repository.
- Anodex integration belongs in an adapter or integration project, not in the
  platform core.
- Keep application-specific behavior out of the platform unless it is clearly
  reusable by more than one application.

## Organization

- apps/ contains consumers of the platform.
- packages/ contains reusable cross-platform packages and SDKs.
- native/ contains native hosts and operating-system adapters.
- docs/ contains maintained project knowledge.
- tests/ contains cross-component tests that do not belong to one package.
- Generated output, downloaded models, logs, credentials, and local runtime
  state must never be committed.
- Maintained source and documentation files must stay at or below 550 physical
  lines. Split a file by responsibility before it reaches that boundary; run
  `scripts/check-source-size.ps1` after structural work.

## Documentation requirements

- Update documentation when behavior, interfaces, security assumptions, or
  folder responsibilities change.
- Record important architecture choices as numbered decision records under
  docs/decisions/.
- Document public protocol fields and compatibility rules before implementing
  them.
- Prefer diagrams and examples when they make a boundary easier to understand.

## Design requirements

- Keep platform services behind explicit interfaces.
- Keep the renderer or application UI away from raw operating-system APIs.
- Treat the protocol as versioned public surface area.
- Make permissions and capabilities explicit; do not rely on hidden ambient
  authority.
- Prefer small, testable modules over framework-specific global state.
- Do not make a migration irreversible until the old implementation and the new
  implementation can be compared.

## Verification

Every implementation change should include the smallest relevant verification:

- unit tests for pure logic;
- protocol compatibility tests for message changes;
- integration tests for host/service boundaries;
- manual verification for native window and operating-system behavior.

Do not claim a feature is complete until its documentation and verification are
also complete.
