# Anodrel

Anodrel is the working name for a reusable native application platform.
It will provide the runtime, security boundaries, platform services, and
communication protocol that multiple desktop applications can use.

Anodex will be the first application built on Anodrel. Anodex is not being
moved into this repository yet; it remains an independent project until the
platform has a stable contract and a working host.

## Status

**Phase:** Foundation and architecture

This repository currently contains the project structure and design documents.
Implementation should begin only after the boundaries in
docs/ARCHITECTURE.md are reviewed and accepted.

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
- Removing every external library or operating-system dependency.
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

The initial implementation is expected to use Rust for the native host and
platform-sensitive code, while existing TypeScript and React applications can
connect through a stable protocol. This is a design direction, not yet a
locked implementation decision; see the decision records for changes.

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

## Working rule

Every substantial architectural change must update the relevant documentation
in the same change. The code, tests, and documentation should describe the same
system at all times.
