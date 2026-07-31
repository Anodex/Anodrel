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
`anodrel-wire`, `anodrel-transport`, `anodrel-bootstrap`,
`anodrel-application`, `anodrel-windows-pipe`, `anodrel-windows-bootstrap`, and the Windows host are
all owned source modules. The host calls User32 and Kernel32 directly for its
window lifecycle and drawing; the pipe adapter uses direct Win32 and CNG APIs
on a worker thread, while the bootstrap adapter uses an explicit Windows child
handle list:

~~~text
cargo fmt --manifest-path native/Cargo.toml --all --check
cargo test --manifest-path native/Cargo.toml
cargo clippy --manifest-path native/Cargo.toml --all-targets -- -D warnings
cargo tree --manifest-path native/Cargo.toml
cargo test --manifest-path native/Cargo.toml -p anodrel-windows-pipe
cargo test --manifest-path native/Cargo.toml -p anodrel-windows-bootstrap
cargo run --manifest-path native/Cargo.toml -p anodrel-windows-host
~~~

The last command is a manual smoke check: an **Anodrel Windows host** window
must show a successful internal `platform.health` response and close normally.
It needs Windows but does not require WebView2. Do not add privileged
capabilities or new third-party runtime dependencies. The named-pipe adapter
already binds a logon-SID DACL and a host-generated credential; the bootstrap
adapter performs one-time delivery through child standard input.

The host can also display the digest-verified, no-script sample application
package described in `docs/APPLICATIONS.md`:

~~~text
cargo run --manifest-path native/Cargo.toml -p anodrel-windows-host -- --application apps/sample/anodrel.application.json
~~~

The window must identify `org.anodrel.sample`, report verified content
integrity, and display the sample text. Close it normally. This is not a
packaged process launcher: publisher trust, executable verification, and
application lifecycle policy remain separate work.

While that window is open, run the same command a second time. It must not
create another application window; it waits at most one second for the primary
window and requests that Windows restore and foreground it. The second process
forwards no data. See `docs/INSTANCE_LIFECYCLE.md` for the exact boundary.

For the quickest Windows smoke test, double-click `start.bat` in the repository
root. It checks for Cargo, builds the host if necessary, validates the sample
package and internal protocol core, completes one owned private IPC loopback,
then opens the Anodrel Startup Lab. It pauses with a clear error if startup
fails. See `docs/STARTUP_LAB.md` for the visual test contract.

### Windows end-to-end development sample

After `npm run build`, run this PowerShell command from the repository root:

~~~powershell
$nodePath = (Get-Command node).Source
$clientPath = (Resolve-Path apps/sample/dist/native-client.js).Path
cargo run --manifest-path native/Cargo.toml -p anodrel-windows-host -- --sample-client $nodePath $clientPath
~~~

It launches the compiled sample through the direct Windows bootstrap adapter.
The sample reads its private standard-input invitation, authenticates to the
real named pipe, calls `platform.health`, and exits with code zero. The host
prints only a safe success summary. This is a development diagnostic—not a
packaged application launcher, trusted content host, or replacement for the
future application-identity policy.

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
