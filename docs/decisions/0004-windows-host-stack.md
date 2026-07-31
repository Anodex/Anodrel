# Decision 0004: The first Windows host uses Rust, Tao, Wry, and WebView2

**Status:** Superseded by Decision 0005

**Date:** 2026-07-31

## Context

Anodrel needs a Windows-native proof host while preserving application
independence from a specific UI or host framework. Existing TypeScript and
React applications should remain viable UI clients, but must not import raw
Windows or webview APIs.

## Decision

The initial Windows host is implemented in Rust. Tao owns native windows and
the event loop; Wry hosts the Windows system WebView2 runtime. The application
boundary is a narrow JSON IPC bridge that maps to the documented Anodrel
protocol. Applications use the SDK transport abstraction rather than Tao, Wry,
WebView2, or framework-specific commands.

The proof host serves only host-controlled HTML from an allowlisted
`anodrel://localhost` custom protocol. It exposes no privileged operation beyond
the existing core protocol operations.

## Consequences

Positive:

- the host uses the Windows system webview instead of bundling Electron;
- protocol and policy logic compile independently from the UI shell;
- a future native UI or alternate transport can reuse the core crate and public
  protocol;
- the bridge handles JSON-compatible data rather than arbitrary host calls.

Tradeoffs:

- the Windows host requires the WebView2 Runtime;
- Tao and Wry versions must be kept compatible and monitored for updates;
- native integration tests and manual window tests are now required for host
  changes.

## Revisit conditions

This proof was superseded when Anodrel adopted a first-party native dependency
policy. It remains a comparison point only; it must not become the production
host.
