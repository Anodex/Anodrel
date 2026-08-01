# Decision 0028: Native UI appearance is portable semantic data

**Status:** Accepted

**Date:** 2026-08-01

## Context

The first Windows UI Lab proved the constrained tree, renderer, pointer input,
accessibility snapshot, and focus traversal. Its renderer initially recognized
specific element IDs to choose a panel or text colour. That works for a fixed
diagnostic, but it makes portable documents depend on a host-private naming
convention and would force every future renderer to repeat that convention.

Adding a theme library, renderer objects, pixel values, or operating-system
handles to the portable UI crate would create the opposite problem: a visual
implementation would leak across the renderer boundary and make future hosts
less independent.

## Decision

`anodrel-ui` carries a small, closed set of semantic appearance roles:

- stacks request a `Plain` or `Raised` surface;
- text requests `Primary`, `Secondary`, or `Accent` prominence; and
- actions request `Neutral` or `Accent` prominence.

Each node starts with its least-emphatic role and has an explicit builder to
select another role. Roles are plain portable Rust values. They do not change
layout, measurement, clipping, accessibility, focus, enabled state, semantic
action identity, or authority.

A native renderer maps the roles to its own palette and drawing rules. It must
not infer presentation from special element-ID strings. The Windows UI Lab is
the first consumer and retains its host-owned dynamic status text as a separate
diagnostic concern.

## Consequences

- documents can express limited visual hierarchy without importing a browser,
  webview, third-party theme engine, or renderer API;
- Windows, Linux, and macOS renderers can choose platform-appropriate visuals
  while interpreting the same structural request; and
- the portable UI contract remains small and easy to validate and test.

## Revisit conditions

Revisit before adding custom colours, typography controls, images, animations,
theme loading, package serialization, untrusted documents, or a public styling
API. Those features need their own resource limits, compatibility rules, and
security review.
