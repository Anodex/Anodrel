# Decision 0005: Production native hosts are first-party modules over OS APIs

**Status:** Accepted

**Date:** 2026-07-31

## Context

Anodrel is intended to be a platform that applications can understand, audit,
and evolve without inheriting framework behavior or a large transitive runtime.
The Windows Tao/Wry proof host validated a window and JSON bridge, but it added
third-party frameworks and runtime libraries that Anodrel would have to track,
trust, and work around.

## Decision

Production Anodrel runtime code may depend only on:

- Anodrel-owned modules;
- the Rust and TypeScript standard libraries; and
- the directly targeted Windows, macOS, or Linux operating-system APIs and SDKs.

Windowing, webview integration, serialization, scheduling, storage, and IPC
layers belong to Anodrel when they are part of the production runtime. A
third-party framework or runtime library is prohibited by default. Any exception
requires a decision record that names the OS API gap, the exact dependency,
security and performance costs, removal plan, and explicit user approval.

Compilers, formatters, test runners, and other development-only tools are not
production runtime dependencies. They remain subject to normal supply-chain
review.

The Tao/Wry/Serde/Time Windows host was removed and replaced by the
Anodrel-owned direct Windows API host in Decision 0006. It must not be restored
or used as a production dependency.

## Consequences

Positive:

- Anodrel owns its native behavior, allocation choices, message flow, and
  operating-system boundaries;
- platform-specific code remains explicit and easier to audit;
- native framework upgrades and transitive runtime churn cannot dictate the
  public protocol or packaging model.

Tradeoffs:

- more platform integration code must be implemented and tested in-house;
- direct OS API bindings require careful safety isolation and extensive manual
  verification;
- feature delivery is slower until the platform primitives mature.

## Revisit conditions

Revisit only when a documented OS API gap prevents a required capability and an
explicitly approved exception has a credible removal path.
