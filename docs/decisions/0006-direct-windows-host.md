# Decision 0006: First production-path Windows host uses owned Win32 modules

**Status:** Accepted

**Date:** 2026-07-31

## Context

Decision 0005 excludes third-party runtime frameworks from Anodrel's shipped
native code. The prior Windows comparison host used Tao, Wry, Serde,
serde_json, and Time. It demonstrated a window and protocol response but did
not meet that dependency policy.

Anodrel needs a small, auditable first production-path host that exercises
window lifecycle and the protocol core without committing the platform to a UI
framework, browser wrapper, serialization framework, or date library.

## Decision

The first Windows host is composed only of Anodrel-owned Rust crates, the Rust
standard library, and direct Win32 APIs:

~~~text
anodrel-json -> anodrel-protocol -> anodrel-core -> anodrel-windows-host
                                                        |
                                                        +-> User32 / Kernel32
~~~

- `anodrel-json` owns strict JSON parsing and serialization for the public
  protocol. It rejects duplicate object keys, malformed Unicode, trailing data,
  and nesting deeper than 64 levels.
- `anodrel-protocol` maps the documented v1 envelope to owned data types and
  ignores unknown additive fields.
- `anodrel-core` performs the 64 KiB encoded-message check before parsing,
  applies host-created capability policy, and formats the small timestamp it
  needs with standard-library time data.
- `anodrel-windows-host` contains the entire raw FFI boundary: class
  registration, window creation, message dispatch, client-area painting, and
  teardown through User32 and Kernel32.

The first window displays an internal `platform.health` response. It has no
webview, rendered application content, inbound application transport, or
privileged operating-system capability. A future UI or transport must have its
own documented contract and threat-model extension before it accepts untrusted
content.

## Consequences

Positive:

- the deployed native dependency graph contains only Anodrel modules;
- Windows calls and unsafe code have one small, inspectable location;
- protocol validation and policy are independently testable without a window;
- the core can be reused by later macOS and Linux adapters.

Tradeoffs:

- Anodrel must build abstractions such as content hosting and IPC itself;
- the direct FFI layer requires manual Windows verification in addition to unit
  tests;
- the current window proves lifecycle and rendering only, not application UI
  integration.

## Revisit conditions

Revisit this structure only when a public service needs a new OS adapter
boundary or a documented transport/UI contract. Replacing it with a third-party
runtime still requires the exception process in Decision 0005 and explicit user
approval.
