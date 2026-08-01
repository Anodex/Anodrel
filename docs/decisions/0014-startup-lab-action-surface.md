# Decision 0014: Show planned actions in a declared pending state

**Status:** Accepted

**Date:** 2026-07-31

## Context

The Startup Lab's design includes an action strip: launch the sample
application, open runtime logs, inspect the verified package, and show runtime
diagnostics. Two of those are backed by work that exists today. The other two
are not — launching a product executable requires verified executable identity,
and a log view requires a logging boundary. Neither is built.

That leaves a choice about what a surface may show. Hiding the unbuilt tiles
would keep every claim true but would misrepresent the platform's intended
shape and would make the surface change layout as capabilities land. Drawing
them as though they worked would be a false claim about a security-relevant
capability, which `docs/THREAT_MODEL.md` and `docs/STARTUP_LAB.md` forbid.

## Decision

Every action tile is shown. Each carries an explicit state:

- **linked** — a capability exists behind it. The tile is drawn at full
  strength, shows a chevron, takes a pointer cursor, and opens a host-owned
  window when clicked.
- **planned** — no capability exists behind it. The tile is dimmed, is labelled
  `PLANNED` where the chevron would be, states the gate it is waiting on in
  place of a description, and does not respond to hover or to a click.

State is data on the tile, not a drawing detail, and hit-testing consults the
same value the renderer does. A unit test asserts that exactly the tiles with a
capability behind them are marked linked, so a tile cannot be enabled by editing
its appearance.

The two linked tiles open host-owned document windows that report facts already
verified during startup: the package's identity, declared content path, verified
digest and limits; and the protocol, transport, and process readings. Neither
introduces a new capability — both display values the host already held.

`ROADMAP.md` tracks each planned tile against the decision that gates it.

## Consequences

Positive:

- the surface communicates the platform's whole intended shape without
  overstating what is built;
- linking a tile later is a data change plus its capability, not a redesign;
- a reader can tell, from the screen alone, which parts of the platform are
  real — which is what a diagnostic surface is for.

Tradeoffs:

- the surface shows controls that do not act, which needs the pending state to
  stay visually unmistakable;
- the pending label and its stated gate must be revisited whenever the roadmap
  moves, or the screen becomes stale in a new way.

## Revisit conditions

Revisit when the last planned tile is linked, or if the strip grows past the
point where a tile can state its gate in a few words. Any tile that would carry
a privileged capability — process launch above all — needs its own threat-model
entry before it moves to linked, not merely a working implementation.
