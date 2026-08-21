# Decision 0026: Native UI accessibility begins with an owned semantic snapshot

**Status:** Accepted; extended by Decision 0075

**Date:** 2026-08-01

## Context

A visible native UI needs stable roles, names, enabled state, and geometry
before it can support operating-system accessibility. Letting an eventual host
adapter rediscover those semantics from pixels or private view state would make
accessibility inconsistent and couple it to one operating system. Shipping an
accessibility framework or exposing a native provider before a bounded model is
established would create a broad, difficult-to-test authority surface.

## Decision

`anodrel-ui` derives a bounded, source-ordered accessibility snapshot from a
validated `UiDocument` and one concrete `UiLayout`. It represents a stack as a
group, text as static text, and an action as a button. Each visible node carries
only its existing element ID, clipped bounds, role, enabled state, and plain
text name where applicable.

Decision 0075 extends each node with its direct visible parent's earlier
source-order index. That preserves the document's declared hierarchy for native
adapters without adding application-defined relations, an operating-system
object, or a mutable view lookup.

The snapshot is portable data. It performs no operating-system call, does not
accept an application package, cannot focus a control, and cannot invoke an
action. Windows UI Automation, Linux AT-SPI, and macOS NSAccessibility adapters
remain future, operating-system-specific work above this layer.

## Consequences

- Accessibility semantics stay consistent with layout and do not depend on
  inspecting pixels or host-private UI state;
- the bounded UI model remains free of third-party accessibility runtimes; and
- a future native adapter has a small, documented input rather than an implicit
  renderer coupling.

## Revisit conditions

Revisit before accepting untrusted UI documents, adding live announcements,
focus or keyboard navigation, relations between nodes, an operating-system
accessibility adapter, or action invocation through an assistive technology.
