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
npm run cli-demo # Run the public-SDK command-line example through the mock host
~~~

The workspace uses TypeScript project references to keep package dependencies
explicit and build them in dependency order. Generated `dist/` folders are not
tracked.

`docs/SDK.md` defines the public TypeScript client, the mock-development
boundary, and the separate Windows named-pipe diagnostic adapter.

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
cargo test --manifest-path native/Cargo.toml -p anodrel-windows-product-session
cargo test --manifest-path native/Cargo.toml -p anodrel-windows-launch
cargo test --manifest-path native/Cargo.toml -p anodrel-product-fixture
cargo test --manifest-path native/Cargo.toml -p anodrel-product-provisioning
cargo test --manifest-path native/Cargo.toml -p anodrel-windows-paths
cargo test --manifest-path native/Cargo.toml -p anodrel-storage
cargo test --manifest-path native/Cargo.toml -p anodrel-windows-storage
wsl -- bash -lc 'source "$HOME/.cargo/env" && cd "/mnt/c/Users/Owner/Desktop/Platform X/native" && CARGO_TARGET_DIR=/tmp/anodrel-linux-target cargo test -p anodrel-linux-storage'
cargo test --manifest-path native/Cargo.toml -p anodrel-windows-crash
wsl -- bash -lc 'source "$HOME/.cargo/env" && cd "/mnt/c/Users/Owner/Desktop/Platform X/native" && CARGO_TARGET_DIR=/tmp/anodrel-linux-target cargo test -p anodrel-linux-crash'
wsl -- bash -lc 'source "$HOME/.cargo/env" && cd "/mnt/c/Users/Owner/Desktop/Platform X/native" && CARGO_TARGET_DIR=/tmp/anodrel-linux-target cargo test -p anodrel-linux-bootstrap'
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

`anodrel-package-tool` is the first-party native authoring and verification
tool for the current static application-package boundary. It shares
`anodrel-application`'s validator and SHA-256 implementation; see
`docs/APPLICATION_TEMPLATE.md`.

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

### Create a first content package

The first-party native package tool creates a new strict plain-text package
without copying host code or generating an executable. From the repository
root, run:

~~~text
cargo run --release --manifest-path native/Cargo.toml -p anodrel-package-tool -- init out/hello-anodrel org.example.hello "Hello Anodrel" "Hello from a native Anodrel package."
~~~

It writes through the exact validator and SHA-256 implementation that the host
uses, then independently reloads its result. It does not sign, install, launch,
or grant anything. Use the `--application` command above with the new manifest
path to open it. See
`docs/APPLICATION_TEMPLATE.md` for its limits, safe failure behaviour, and
focused verification command.

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

To exercise the full bounded session-owned multi-window flow, double-click
`start-multi-window-template.bat` in the repository root. It generates and
builds a disposable first-party Rust executable, then opens it through the
fixed development route. In order, activate **Open secondary window** in the
primary, **Replace this window** in the secondary, and **Close secondary and
finish** in that same secondary. It creates no certificate, package, installer,
or machine policy. See `docs/NATIVE_MULTI_WINDOW_TEMPLATE.md`.

To exercise explicit v2 secondary scrolling, double-click
`start-scroll-window-template.bat` in the repository root. It generates and
builds a disposable first-party Rust executable, then opens it through the
fixed development route. Activate **Open scrollable secondary window** in the
primary. In the secondary, scroll until **Reveal updated scroll document** is
visible and activate it; then scroll the replacement document until **Close
secondary and finish** is visible and activate it. The host retains each
scroll position and all native scroll input. The helper creates no certificate,
package, installer, or machine policy. See `docs/NATIVE_SCROLL_WINDOW_TEMPLATE.md`.

To exercise every typed targetless session-window control, double-click
`start-window-controls-template.bat` in the repository root. It generates and
builds a disposable first-party Rust executable, then opens it through the
fixed development route. Advance **Set host-composed title**, **Resize client
area**, **Maximise window**, **Restore window**, **Request foreground
attention**, **Enter fullscreen**, **Return to windowed**, and **Complete
window-controls session** in order. The host retains native state and the
foreground result; the helper creates no certificate, package, installer, or
machine policy. See `docs/NATIVE_WINDOW_CONTROLS_TEMPLATE.md`.

To exercise submit-time native form entry, double-click
`start-form-template.bat` in the repository root. It generates and builds a
disposable first-party Rust executable, then opens it through the fixed
development route. Enter any non-secret test text in **Name**, then activate
**Submit form**. The app takes one whole-surface snapshot only after that
semantic action and closes without displaying, writing, or retaining the value.
It creates no certificate, package, installer, or machine policy. See
`docs/NATIVE_FORM_TEMPLATE.md`.

To exercise explicit visible status changes, double-click
`start-live-status-template.bat` in the repository root. It generates and
builds a disposable first-party Rust executable, then opens it through the
fixed development route. Start Narrator after the window opens if you are
checking announcements, then activate **Publish polite result**, **Publish
assertive result**, and **Complete status session** in order. The application
cannot learn whether a status was announced. It creates no certificate,
package, installer, or machine policy. See `docs/NATIVE_LIVE_STATUS_TEMPLATE.md`.

To exercise one first-party native notification, double-click
`start-notification-template.bat` in the repository root. It generates and
builds a disposable Rust executable, then opens it through the fixed three-grant
development route. Watch for the fixed notification while its window stays open
for five seconds. A successful run means the host accepted the request, not
that the process knows a person saw it. The helper creates no certificate,
package, installer, or machine policy. See
`docs/NATIVE_NOTIFICATION_TEMPLATE.md`.

To exercise retained selected-output writing, double-click
`start-file-write-template.bat` in the repository root. It generates and
builds a disposable first-party Rust executable, then opens it through the
fixed four-grant development route. Choose a fresh `.txt` file and check that
it contains the one documented fixed line; run it again and cancel to confirm
cleanup. The helper creates no certificate, package, installer, or machine
policy. See `docs/NATIVE_FILE_WRITE_TEMPLATE.md`.

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

### Startup and memory readings

~~~text
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --startup-report apps/sample/anodrel.application.json
~~~

Runs the startup checks, prints one JSON object, and exits without opening a
window. Use it when changing anything on the startup path. Run it more than
once: the first run after a build is several times slower because the
executable is still being read from disk.

See `docs/PERFORMANCE.md` for what the figures exclude and what has to be true
before either is compared with another runtime.


## Direct Linux Wayland lab

The Linux Wayland Lab is a development-only presentation check, separate from
the Windows host and the Linux invited-child foundation. On a little-endian
Linux Wayland desktop with XDG_RUNTIME_DIR and WAYLAND_DISPLAY set by the
session, run:

~~~text
cargo run --release --manifest-path native/Cargo.toml -p anodrel-linux-window-lab
~~~

It opens one fixed-size branded surface. On a desktop with a pointer, clicking
the highlighted lower panel once shows a completed appearance; closing the
desktop window then exits normally. It does not load an application, start a
Linux client, expose an SDK, or claim a packaged Linux host. See
docs/LINUX_WINDOWING.md.

## Linux child/view Session Lab

The separate Linux Session Lab proves that one held first-party invited child
and one fixed Wayland Lab view finish together under one host lifetime. On the
same local little-endian Wayland desktop, run:

~~~text
scripts/start-linux-session-window-lab.sh
~~~

It builds only the fixed first-party held child, opens the standard Linux Lab,
and supplies that exact generated executable. Close the desktop view and the
child is stopped and joined before the command returns. This still does not
load application content, expose a Linux host SDK, or claim a product Linux
host. See docs/LINUX_WINDOW_SESSIONS.md.

## Diagnostics and product fixture

The compiled diagnostics, native end-to-end samples, screen-reader check, and development product fixture are maintained in [Development diagnostics](DEVELOPMENT_DIAGNOSTICS.md).

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
