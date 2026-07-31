# Anodrel

Anodrel is a reusable native application platform.
It will provide the runtime, security boundaries, platform services, and
communication protocol that multiple desktop applications can use.

Anodex will be the first application built on Anodrel. Anodex is not being
moved into this repository yet; it remains an independent project until the
platform has a stable contract and a working host.

## Status

**Phase:** Foundation implementation

The first implementation slice defines the transport-neutral protocol, client
SDK, mock host, sample application, shared contract tests, a bounded native
transport engine, an authenticated direct Windows named-pipe adapter, and an
owned direct Windows host. The Windows host proves the native window lifecycle
and protocol core without a runtime framework or a webview. It exposes no
privileged operating-system capability and does not yet accept application
content.

## Goals

- Create a reusable native desktop runtime.
- Keep application logic independent from Electron and any single UI toolkit.
- Provide explicit, versioned interfaces for windows, storage, permissions,
  notifications, dialogs, processes, and other platform services.
- Make security boundaries understandable, testable, and documented.
- Let future applications reuse the platform without copying Anodex internals.
- Keep Anodex and Anodrel in separate repositories with separate Git history.

## Non-goals

- Rebuilding an operating system.
- Rebuilding a browser engine.
- Rewriting all of Anodex in one large migration.
- Avoiding every external development tool; the restriction applies to shipped
  runtime dependencies, not compilers and test tooling.
- Designing a platform around Anodex-only concepts.

## Planned architecture

~~~text
Application
    │
    │ versioned protocol / SDK
    ▼
Platform Core
    │
    │ platform service interfaces
    ▼
Native Host
    │
    ├── Windows
    ├── macOS
    └── Linux
~~~

The current Windows host uses Anodrel-owned modules over direct User32 and
Kernel32 APIs. The direct pipe adapter is restricted to the current Windows
logon session and requires host-created credentials. Existing TypeScript and
React applications remain UI clients through the SDK rather than importing
native APIs; their private bootstrap and content-hosting boundary remain a
separate documented step.

## Repository map

~~~text
Anodrel/
├── apps/                 # Applications built on the platform
├── packages/             # Cross-platform TypeScript packages and SDKs
├── native/               # Native host and platform adapters
├── docs/                 # Architecture, guides, and decision records
├── scripts/              # Development and release helpers
├── tests/                # Cross-component and integration tests
├── AGENTS.md             # Instructions for coding agents
├── ROADMAP.md            # Sequenced project work
└── README.md             # Project entry point
~~~

## Documentation

- AGENTS.md — rules for making changes safely.
- ROADMAP.md — current phases and acceptance gates.
- docs/ARCHITECTURE.md — system boundaries and data flow.
- docs/DEVELOPMENT.md — local workflow and verification.
- docs/decisions/ — durable decisions and their reasoning.

`docs/TRANSPORT.md` defines the native frame and session contract.
`docs/PERFORMANCE.md` defines how Electron comparisons will be measured.

The repository's GitHub Pages landing page lives in `docs/index.html` and uses
only hand-authored HTML and CSS.

## Working rule

Every substantial architectural change must update the relevant documentation
in the same change. The code, tests, and documentation should describe the same
system at all times.

## Current foundation commands

After installing the workspace dependencies, run:

~~~text
npm run check
npm test
npm run demo
~~~

See docs/DEVELOPMENT.md for prerequisites and the expected verification order.

The public interface and security baseline are documented in docs/PROTOCOL.md
and docs/THREAT_MODEL.md.
