# Anodrel Development Guide

## Current state

Anodrel has a TypeScript workspace for the versioned protocol, SDK, mock host,
sample application, and contract tests. It also has a bounded wire and
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
`anodrel-application`, `anodrel-session-policy`, `anodrel-windows-pipe`, `anodrel-windows-bootstrap`,
`anodrel-paths`, `anodrel-windows-policy`, `anodrel-windows-launch`,
`anodrel-windows-paths`, `anodrel-credentials`,
`anodrel-windows-credentials`, `anodrel-ui`, and the Windows host are all source modules. The
host calls User32 and Kernel32 directly for its
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
cargo test --manifest-path native/Cargo.toml -p anodrel-windows-policy
cargo test --manifest-path native/Cargo.toml -p anodrel-session-policy
cargo test --manifest-path native/Cargo.toml -p anodrel-windows-registered-session
cargo test --manifest-path native/Cargo.toml -p anodrel-windows-launch
cargo test --manifest-path native/Cargo.toml -p anodrel-windows-paths
cargo test --manifest-path native/Cargo.toml -p anodrel-windows-credentials
cargo test --manifest-path native/Cargo.toml -p anodrel-ui
cargo test --manifest-path native/Cargo.toml -p anodrel-ui-document
cargo test --manifest-path native/Cargo.toml -p anodrel-ui-session
cargo run --release --manifest-path native/Cargo.toml -p anodrel-perf-lab -- --iterations 5000
cargo run --release --manifest-path native/Cargo.toml -p anodrel-perf-lab -- --windows-pipe --iterations 5000
cargo run --manifest-path native/Cargo.toml -p anodrel-windows-host
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --ui-lab
~~~

The direct host command without an argument is a manual smoke check: an
**Anodrel Windows host** window must show a successful internal
`platform.health` response and close normally. `--ui-lab` is a separate manual
check for the owned UI foundation: hover actions for the hand cursor, click one,
and confirm that the screen reports only its semantic ID. Press Tab and
Shift+Tab to move the focus ring through its actions; Enter must report the
focused action's same semantic ID.
It needs Windows but does not require WebView2. Do not add privileged
capabilities or new third-party runtime dependencies. The named-pipe adapter
already binds a logon-SID DACL and a host-generated credential; the bootstrap
adapter performs one-time delivery through child standard input.

To preview one explicit v1 UI document through the same native renderer, run:

~~~text
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --ui-preview path-to-document.json
~~~

The file must be a valid bounded `anodrel.ui.document.v1` document. Hover its
actions and use Tab/Shift+Tab/Enter to exercise the local input path. This is a
developer diagnostic only, not an application session. See `docs/UI_PREVIEW.md`.

The host can also display the digest-verified, no-script sample application
package described in `docs/APPLICATIONS.md`:

~~~text
cargo run --manifest-path native/Cargo.toml -p anodrel-windows-host -- --application apps/sample/anodrel.application.json
~~~

The window must identify `org.anodrel.sample`, report verified content
integrity, and display the sample text. Close it normally. This is not a
user-facing packaged process launcher: the host-only registered launch service
is deliberately separate from this display path until a signed application and
machine policy record are provisioned.

While that window is open, run the same command a second time. It must not
create another application window; it waits at most one second for the primary
window and requests that Windows restore and foreground it. The second process
forwards no data. See `docs/INSTANCE_LIFECYCLE.md` for the exact boundary.

To verify the multi-window lifecycle, run:

~~~text
cargo run --manifest-path native/Cargo.toml -p anodrel-windows-host -- --window-lab
~~~

Two **Anodrel Window Lab** windows must open. Closing one must leave the other
open; closing the final window must exit the host. See
`docs/WINDOW_LIFECYCLE.md`.

For the quickest Windows smoke test, double-click `start.bat` in the repository
root. It checks for Cargo, builds the host if necessary, validates the sample
package and internal protocol core, completes one private IPC loopback,
then opens the Anodrel Startup Lab. It pauses with a clear error if startup
fails. See `docs/STARTUP_LAB.md` for the visual test contract.

`start.bat` builds in release. The Startup Lab composes every frame in software,
and an unoptimised build is roughly ten times slower — far too slow to hold the
reveal's frame rate. When running the host by hand for a visual check, pass
`--release` for the same reason:

~~~text
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --showcase apps/sample/anodrel.application.json
~~~

The frame budget is enforced by a test that only runs in an optimised build, so
include a release test run when changing anything the renderer touches:

~~~text
cargo test --release --manifest-path native/Cargo.toml
~~~

Rendering itself is tested headless by asserting on pixels, so most visual
regressions surface in `cargo test` without opening a window. See
`docs/RENDERER.md` for the renderer's API and its testing approach.

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

To see the same authenticated client replace a host-controlled direct native
window, run:

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --sample-ui-client $nodePath $clientPath
~~~

The **Anodrel UI Session Lab** opens with a host-owned waiting screen, then
replaces it with the document the client submits after authentication. Click
the delivered action (or focus it with Tab and press Enter) to complete its
authenticated `ui.events.read` round trip. The sample then requests
`session.close` for that same host-owned session, and the lab window closes.
The action carries only its revision and semantic ID; it cannot invoke a native
operation. See `docs/UI_SESSION_LAB.md`.

To exercise the actual host-owned open or save picker in that same session,
replace `--sample-ui-client` with `--sample-ui-file-client` or
`--sample-ui-save-client`. Both commands accept only the strict sample filters;
the save diagnostic never writes the selected destination.

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
