# Decision 0139: Finish the Windows reference release before other hosts

**Status:** Accepted

**Date:** 2026-08-30

## Context

Anodrel has a substantial direct Windows host and smaller Linux foundation
labs. Continuing platform-specific work in parallel would create three
incomplete versions of the same contracts, consume verification effort, and
blur the point at which the platform is ready for a real application.

Windows is the first selected operating system, has the strongest native,
accessibility, diagnostic, and product-session evidence, and is where the
remaining distribution decisions have immediate value.

## Decision

Windows is the reference platform. It must meet the release gates recorded in
`docs/WINDOWS_RELEASE.md` before new Linux-specific or macOS-specific feature
work begins.

A portable change may proceed during this period only when it directly closes a
Windows gate. Every platform-specific feature still requires its own contract,
decision record where appropriate, tests, and native verification; Windows
priority does not lower those boundaries.

## Consequences

Positive:

- One host reaches a genuinely usable, measurable release state first.
- Distribution, signing, accessibility, performance, and native interaction
  evidence accumulates against one concrete reference rather than being split.
- Linux and macOS can reuse proven portable contracts and learn from a shipped
  Windows route.

Tradeoffs:

- Linux and macOS foundation labs pause before they become application hosts.
- A portable subsystem without a direct Windows purpose waits even when it
  would be useful later.
- The release plan now needs explicit operator decisions for Windows signing,
  packaging, installation, and updates before it can close completely.

## Revisit conditions

Revisit if a Windows release gate depends on a platform-neutral subsystem that
cannot reasonably be designed against the Windows host alone, or after the
Windows reference release has recorded its acceptance evidence.
