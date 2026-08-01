# Decision 0055: Windows high contrast uses direct system colours

**Status:** Accepted

**Date:** 2026-08-01

## Context

The portable Anodrel UI foundation has semantic appearance roles, focus, and a
visible accessibility snapshot, but direct software-rendered Windows surfaces
previously always used the authored Anodrel palette. A user who enables Windows
high contrast needs a host-selected accessible colour mapping. Shipping a theme
framework or allowing application documents to select native colours would
expand the runtime and blur the portable/native boundary.

## Decision

Add a narrow Windows-only appearance adapter. It reads only the current
high-contrast flag through `SystemParametersInfoW` and six documented system
colours through `GetSysColor`. The Windows UI Lab and UI Session Lab replace
their host palette with those colours only while high contrast is enabled.

The adapter exposes no setting mutation, subscription, application API,
protocol operation, registry data, path, or user identity. The portable UI
document continues to express only its existing semantic roles.
The native window procedure may schedule a fresh paint for an existing UI Lab
or UI Session Lab after Windows broadcasts `WM_SETTINGCHANGE`; that is a
window-lifecycle repaint, not an application-facing setting subscription.

## Consequences

Positive:

- the first direct native interactive surfaces honour the user's Windows
  high-contrast choice without a third-party runtime;
- high-contrast colours stay a Windows adapter concern and do not leak into the
  portable UI model; and
- the renderer remains Anodrel's software renderer and its tests stay
  deterministic through an explicit palette seam.

Tradeoffs:

- appearance is sampled at paint time, with a Windows settings broadcast only
  scheduling a fresh paint for the two existing interactive host views;
- Startup Lab and package text surfaces remain on their authored presentation
  until each gains an equally reviewed mapping; and
- operating-system assistive-technology object providers remain a separate
  future boundary.

## Revisit conditions

Revisit before adding live setting notifications, a user-selectable Anodrel
theme, dynamic assets, Windows UI Automation, or macOS/Linux appearance
adapters.
