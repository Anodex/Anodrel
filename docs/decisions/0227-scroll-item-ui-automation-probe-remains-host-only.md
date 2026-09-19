# Decision 0227: ScrollItem UI Automation probing remains host-only

**Status:** Accepted

**Date:** 2026-09-19

## Context

Anodrel's scroll-item provider tests and Windows-host tests establish the
bounded `IScrollItemProvider` route, but they cannot prove that a real Windows
UI Automation client receives the compiled off-screen item, accepts its
`ScrollIntoView` request, and sees a fresh visible publication. The manual
ScrollItem procedure remains important for Narrator and Inspect behavior, but
its client/provider portion is repeatable and should not depend solely on a
person.

Exposing UI Automation or retained scroll state to an application would break
Decisions 0097 and 0098. A generic harness accepting targets, coordinates,
documents, or scroll requests would create the same broad desktop-inspection
and input surface those decisions exclude.

## Decision

Add a development-only `--uia-scroll-item-probe` route. It creates one fixed
host-owned UI Lab window. A private host MTA client finds only the compiled
`ui.lab.scroll.exercise-9` button, verifies its initial off-screen and
non-Invoke state, calls its standard `IScrollItemPattern::ScrollIntoView`
method, then requires a fresh provider publication to make that same element
visible and still non-Invoke.

The direct-client adapter adds only the exact Windows constants, ABI, and
methods necessary to prepare the preselected ScrollItem interface and read the
standard off-screen property. It provides no general tree, target, geometry,
scroll, or result API to an application or SDK.

## Consequences

- One repeatable real-Windows check now covers the ScrollItem provider/client
  bridge and the host's retained reveal path.
- The full accessibility suite gains an eighth fixed direct-client probe.
- Manual acceptance remains required for spoken output, visual highlighting,
  authenticated v2 sessions, already-visible behavior, and nested navigation.

## Revisit conditions

Revisit before accepting an application-selected target, document, coordinate,
viewport, item, request, alignment, geometry, visibility result, scroll
position, callback, listener, non-Windows equivalent, or broader UI Automation
client surface.
