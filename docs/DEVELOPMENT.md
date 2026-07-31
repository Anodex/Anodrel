# Anodrel Development Guide

## Current state

Anodrel has a TypeScript workspace for the versioned protocol, SDK, mock host,
sample application, and contract tests. It also has an owned bounded wire and
host-session engine plus a direct Windows host for native window lifecycle and
the core protocol handlers.

The native-host toolchain, packaging, and release commands will be documented
after their decisions are recorded. Do not invent those commands.

## Foundation workspace

Prerequisites:

- Node.js 22 or newer;
- npm 10 or newer.
- Rust 1.95 or newer for the direct Windows host.

Install dependencies from the repository root:

~~~text
npm install
~~~

Use the following commands while working on the foundation packages:

~~~text
npm run check   # Type-check and build all referenced projects
npm test        # Run protocol compatibility tests against the mock host
npm run demo    # Run the sample application through the public SDK
~~~

The workspace uses TypeScript project references to keep package dependencies
explicit and build them in dependency order. Generated `dist/` folders are not
tracked.

## Direct Windows host

The Rust workspace is under `native/`. It has no third-party runtime
dependencies: `anodrel-json`, `anodrel-protocol`, `anodrel-core`,
`anodrel-wire`, `anodrel-transport`, `anodrel-windows-pipe`, and the Windows
host are all owned source modules. The host calls User32 and Kernel32 directly
for its window lifecycle and drawing; the pipe adapter uses direct Win32 and
CNG APIs on a worker thread:

~~~text
cargo fmt --manifest-path native/Cargo.toml --all --check
cargo test --manifest-path native/Cargo.toml
cargo clippy --manifest-path native/Cargo.toml --all-targets -- -D warnings
cargo tree --manifest-path native/Cargo.toml
cargo test --manifest-path native/Cargo.toml -p anodrel-windows-pipe
cargo run --manifest-path native/Cargo.toml -p anodrel-windows-host
~~~

The last command is a manual smoke check: an **Anodrel Windows host** window
must show a successful internal `platform.health` response and close normally.
It needs Windows but does not require WebView2. Do not add privileged
capabilities or new third-party runtime dependencies. The named-pipe adapter
already binds a logon-SID DACL and a host-generated credential; the next step is
private invitation delivery to a launched application, not a public pipe name or
client-provided context.

## Working process

1. Start from an issue or a written task with a clear acceptance condition.
2. Read the root documentation and relevant decision records.
3. Keep changes within the correct repository and layer.
4. Update architecture or decision documentation when a boundary changes.
5. Add tests for protocol and security-sensitive behavior.
6. Run the documented verification commands.
7. Review the Git diff for unrelated files, secrets, generated output, and
   accidental Anodex coupling.

## Change categories

### Architecture changes

Update docs/ARCHITECTURE.md and add a numbered decision record under
docs/decisions/ when the change affects ownership, boundaries, protocol,
security, or supported platforms.

### Protocol changes

Document the schema, compatibility rule, migration behavior, and tests before
changing an existing message. Prefer additive changes over breaking changes.

### Native host changes

Document the operating-system behavior, permission requirements, failure modes,
and cleanup behavior. Include manual verification instructions.

### Application changes

Keep application-specific logic in apps/ or the consuming application. Do not
put product behavior into the platform core merely because it is convenient.

## Git hygiene

- Keep Anodrel commits separate from Anodex commits.
- Do not use the Anodex repository as a submodule or working directory shortcut
  during the foundation phase.
- Do not commit secrets, credentials, downloaded models, installers, logs, or
  generated build output.
- Use focused commits that leave the repository understandable at each step.

## Documentation standard

Documentation should answer:

- What does this component own?
- What does it deliberately not own?
- Who calls it?
- What can fail?
- What security assumptions does it make?
- How is it tested?
- How can it be replaced later?
