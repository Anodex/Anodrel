# Decision 0039: Scroll documents use a new exact format version

**Status:** Accepted

**Date:** 2026-08-01

## Context

The owned UI model now has a bounded `Scroll` viewport, but the established
`anodrel.ui.document.v1` interchange is intentionally exact and contains no
scroll-node representation. Adding fields or a node kind to that identifier
would make compatibility ambiguous.

## Decision

The first external scroll form is `anodrel.ui.document.v2`. It retains every
v1 node object unchanged and adds one exact `scroll` object with a valid `id`
and exactly one `child` node. Scroll position remains host-owned runtime state
and is never encoded. Version 1 continues to reject a `scroll` node; version 2
does not infer or accept later extensions.

## Consequences

- v1 consumers retain their existing strict behavior;
- v2 can represent the complete current owned model without a browser-style
  style or event surface; and
- document replacement, session delivery, and hosts must opt into v2 through
  explicit compatibility tests rather than receiving it accidentally.

## Revisit conditions

Revisit before adding scrollbars, input-policy data, persistent positions,
horizontal scrolling, node attributes, or another external UI tree feature.
