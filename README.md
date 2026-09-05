# Anodrel

Anodrel is a native-first platform for secure, modular desktop applications. It
gives applications a shared foundation of direct operating-system services,
explicit permissions, and versioned local IPC—without shipping a browser runtime.

## What it is

- A reusable desktop platform, beginning with a direct Windows host.
- A security-focused boundary between application code and operating-system
  services: every capability is explicit, bounded, and documented.
- A modular native foundation built on platform APIs rather than a bundled
  browser engine, webview, or Node.js runtime.

It is not an application, an Anodex source mirror, or a finished Electron
replacement. Anodex is planned as its first consumer but remains an independent
project until Anodrel has a stable contract and a shipped reference host.

## Current position

**Phase:** Foundation implementation

Windows is Anodrel's reference platform and is about 72% through its release
goal. The broader Windows, Linux, and macOS programme is about 30% complete.
Linux has development foundations but no application host or product path;
macOS has not begun. Both wait for the Windows reference release.

The Windows foundation already includes authenticated local transport, direct
native windows, semantic UI documents, native services, owned rendering,
accessibility, and owned installer/update foundations. The principal remaining
release work is signed end-to-end acceptance, production certificate custody,
release operation, and the reusable controls justified by the first real app.

For the detailed current state, evidence, and open release gates, see
[Foundation status](docs/FOUNDATION_STATUS.md) and
[Windows release readiness](docs/WINDOWS_RELEASE.md).

## Design principles

- Use direct Windows, Linux, and macOS APIs; ship no third-party desktop runtime.
- Keep services behind documented, versioned interfaces and explicit grants.
- Keep native authority in the host, not in application UI or the protocol.
- Make performance, lifecycle, and security limits measurable and testable.
- Prefer small focused modules and source files at or below 550 lines.
- Keep Anodrel independent from Anodex and preserve separate Git history.

## Architecture

~~~text
Application
    │ documented SDK / protocol
    ▼
Platform Core
    │ service interfaces
    ▼
Native Host
    │ operating-system APIs
    ▼
Windows / Linux / macOS
~~~

The current Windows host uses direct User32, Kernel32, GDI, Shell32, WinHTTP,
Credential Manager, UI Automation, and Authenticode boundaries behind Anodrel
modules. Applications submit versioned requests and semantic UI documents; they
do not receive raw handles, ambient filesystem authority, a browser bridge, or
operating-system control.

## Repository map

~~~text
Anodrel/
├── apps/                 # Platform consumers
├── packages/             # Reusable SDKs and cross-platform packages
├── native/               # Native hosts, tools, and OS adapters
├── docs/                 # Maintained knowledge and decisions
├── scripts/              # Development and release helpers
├── tests/                # Cross-component tests
├── AGENTS.md             # Repository rules for contributors and agents
├── ROADMAP.md            # Delivery order and acceptance gates
└── README.md             # Project entry point
~~~

## Getting started

Install the workspace prerequisites described in
[Development](docs/DEVELOPMENT.md), then run:

~~~text
npm run check
npm test
npm run demo
~~~

On Windows, double-click `start.bat` at the repository root to build and open
the Anodrel Startup Lab. It runs the sample package and a temporary private IPC
health loopback before displaying the native visual smoke test.

The additional root `start-*-template.bat` scripts build bounded first-party
native templates. They exercise specific host capabilities—menus, multiple
windows, scrolling, window controls, and forms—without a webview, Node.js,
product package, or machine-policy change.

## Key documentation

- [Roadmap](ROADMAP.md) — delivery order and platform milestones.
- [Foundation status](docs/FOUNDATION_STATUS.md) — current implementation and
  remaining Windows release work.
- [Architecture](docs/ARCHITECTURE.md) — layer boundaries and responsibilities.
- [Protocol](docs/PROTOCOL.md) — public compatibility and security rules.
- [Windows release readiness](docs/WINDOWS_RELEASE.md) — release gates,
  evidence, and decisions that need product authority.
- [Development](docs/DEVELOPMENT.md) — local setup and verification sequence.
- [Performance](docs/PERFORMANCE.md) — measurable performance policy.
- [Accessibility](docs/ACCESSIBILITY.md) — Windows UI Automation boundary.
- [Decisions](docs/decisions/README.md) — recorded architectural choices.

## Working rule

Every substantial change keeps code, tests, decisions, and documentation in
agreement. Generated output, credentials, logs, downloaded models, and local
runtime state are never committed.
