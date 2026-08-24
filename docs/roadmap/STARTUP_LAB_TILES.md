# Startup Lab action tiles

The Startup Lab shows every action the platform intends to offer, each in a
declared **linked** or **planned** state (Decision 0014). This table is the
list to work through: a tile moves to linked when its documented host operation
exists and — where it is privileged — has a threat-model entry. Linking a tile
is then a data change plus its operation, not a redesign.

| Tile | State | Gate to link it |
| --- | --- | --- |
| Development Fixture | **Resolved at run time** | Its state is no longer a constant. A verification-only preflight — machine record, locked digest revalidation, Authenticode, publisher fingerprint — decides it before the surface opens. With the development fixture of Decision 0061 provisioned it is live and reads *Development only, not a product*; on any other machine it stays *Not provisioned*, dimmed, and inert. It is deliberately not called a product launch, and there is no product-launch tile: that waits on the deferred signing and packaging decision. |
| Open Logs | **Linked** | Done. Shows only the bounded typed host events defined by `docs/LOGGING.md`; it exposes no application text, persistence, export, or capability. |
| Inspect Package | **Linked** | Done. Displays facts already verified at startup; introduces no capability. |
| Runtime Diagnostics | **Linked** | Done. Displays this process's own readings; introduces no capability. |

Adding a tile beyond these four requires the same treatment: show it planned
with its gate stated, and link it only once its underlying operation is real.
