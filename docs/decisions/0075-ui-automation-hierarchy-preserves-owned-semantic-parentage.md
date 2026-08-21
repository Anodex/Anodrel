# Decision 0075: UI Automation hierarchy preserves owned semantic parentage

**Status:** Accepted

**Date:** 2026-08-21

## Context

The first Windows UI Automation provider deliberately published a flat list.
Its portable snapshot was source ordered but carried no parentage, so publishing
`Group` containers would have put their children beside them. A screen reader
would then encounter an empty group rather than a meaningful structural
container. Filtering groups was the honest temporary boundary, but it loses the
layout structure a nested native surface has already declared.

Hierarchy must not become a new application-controlled accessibility language,
a mutable view lookup, or a general relation framework. The existing validated
document already owns the only structure Anodrel can truthfully publish.

## Decision

Each visible `UiAccessibilityNode` carries an optional parent source-order
index. The root visible node has no parent; every other node names its direct,
earlier visible ancestor. The snapshot remains a bounded, immutable preorder
walk of the validated document and concrete layout. It still carries only the
existing ID, role, name, clipped bounds, and enabled flag, and it still makes no
operating-system call.

`anodrel-windows-accessibility` copies that direct parent index beside its
otherwise pure UI Automation mapping. `anodrel-windows-uia` preserves every
mapped node, including `Group`, and derives immutable parent and child lists
once while it builds a provider tree. Fragment navigation then reports only
direct `Parent`, `FirstChild`, `LastChild`, `NextSibling`, and
`PreviousSibling` relationships. The window root owns the top-level nodes;
Windows owns the window root's parent.

Hit testing continues to return the deepest mapped node containing a point. If
same-level painted bounds overlap, later source order wins as the renderer's
paint order already does. A non-interactive `Group` gains no focus target,
pattern, action, or value merely by becoming structural.

This is structural reading only. No application can supply an accessibility
parent, override a relationship, read the published tree, detect an assistive
technology, receive a callback, or learn focus. No UI Automation structure
event, relation property, live provider lookup, protocol field, capability,
or document-format version is added. A new query can see a later immutable
publication, but an older provider remains its original snapshot.

## Consequences

Positive:

- screen readers and UI Automation clients can understand nested Anodrel
  layouts as real groups and children rather than a misleading flat sequence;
- structure remains derived from the exact validated document and layout that
  the host renders; and
- navigation is small pure logic over immutable vectors, so it can be tested
  without COM or a native window.

Tradeoffs:

- the portable snapshot gains one structural field that every future platform
  adapter must preserve or intentionally map;
- a group with no name may still be announced as an unnamed group, which is
  truthful because the portable model declares no group label; and
- document replacement does not raise a structure event, so automation clients
  discover a new hierarchy only through their ordinary fresh query paths.

## Alternatives considered

**Keep filtering groups.** Rejected. It leaves nested layouts without their
declared structure and prevents a screen reader from learning relationships the
host already knows.

**Let applications provide arbitrary accessibility parents or relations.**
Rejected. It would create a second, potentially inconsistent hierarchy and a
new untrusted surface outside the validated UI model.

**Publish a mutable provider that follows live view state.** Rejected. It
would couple COM callers to the host registry and make navigation race document
replacement. Immutable publications make the provider lifetime explicit.

## Revisit conditions

Revisit before adding named groups, labelled-by/described-by relations,
structure events, live regions, selection, text ranges, application-visible
accessibility state, or a non-Windows adapter with materially different
hierarchy requirements.
