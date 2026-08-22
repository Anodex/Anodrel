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

To exercise submit-time native form entry, double-click
`start-form-template.bat` in the repository root. It generates and builds a
disposable first-party Rust executable, then opens it through the fixed
development route. Enter any non-secret test text in **Name**, then activate
**Submit form**. The app takes one whole-surface snapshot only after that
semantic action and closes without displaying, writing, or retaining the value.
It creates no certificate, package, installer, or machine policy. See
`docs/NATIVE_FORM_TEMPLATE.md`.

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

### Crash records

Two routes exercise the host's crash-record path. Run them after changing
anything on the panic-containment path, the window registry, or the record
format:

~~~text
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --crash-report-selftest
cargo run --manifest-path native/Cargo.toml -p anodrel-windows-host -- --crash-selftest-panic
~~~

The first writes one record and exits, confirming the location and format. The
second opens the UI Lab and raises a real panic inside its first paint; the
process must exit **without aborting**, and the new record must read
`surface=ui-lab` rather than `unknown` — that value is what proves the crash was
classified against a live window.

The second route is debug-only, which is why it has no `--release`. Nothing a
user runs can be asked to fault. Neither route is part of `cargo test`, and no
automated test writes to the real record location. See `docs/CRASH_REPORTS.md`
for what a record may contain and what this deliberately does not catch.

### Compiled native development probe

This is the smallest real child path with no Node.js process. It is a
development diagnostic, not a product launcher: you explicitly name the
executable, the host does not verify it, and the probe has exactly one fixed
behaviour (`platform.health`). From the repository root:

~~~powershell
cargo build --release --manifest-path native/Cargo.toml -p anodrel-native-client-sample
$clientPath = (Resolve-Path native/target/release/anodrel-native-client-sample.exe).Path
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --native-sample-client $clientPath
~~~

It prints **Anodrel native development probe completed successfully.** and
exits without opening a window. Its only client dependencies are Anodrel crates,
the Rust standard library, and direct Windows APIs. A safe nonzero child stage
does not expose the private invitation, named pipe, token, or native error.

### Compiled native HTTPS diagnostic

This separate no-window diagnostic tests the direct WinHTTP boundary through
the authenticated protocol. It is not a general network client or product
launcher: the host grants the selected child only `network.fetch` against its
compiled `example.com:443` policy, while the first-party child requests only
`https://example.com/`. It accepts no URL or other network option from the
command line and never prints or retains the response text.

For the quickest check, double-click `start-network-diagnostic.bat` from the
repository root. It builds the host and child in release mode, runs the fixed
route, and pauses with a safe error if the diagnostic cannot complete. It does
not alter certificate, proxy, or machine-policy settings.

~~~powershell
cargo build --release --manifest-path native/Cargo.toml -p anodrel-native-network-client-sample
$networkClientPath = (Resolve-Path native/target/release/anodrel-native-network-client-sample.exe).Path
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --native-network-sample-client $networkClientPath
~~~

With ordinary outbound Internet access, it prints **Anodrel native HTTPS
development diagnostic completed successfully.** and exits without opening a
window. An unavailable network, TLS failure, blocked endpoint, or invalid
response causes a safe nonzero diagnostic failure and must not be worked around
by changing TLS, proxy, redirect, or certificate policy. Regular templates,
Node samples, the product fixture, and installed sessions receive no network
service.

### Compiled native UI-session diagnostic

This development diagnostic extends the compiled health probe through one
host-owned native window. It uses the same private bootstrap and direct Windows
pipe adapter, then replaces the waiting document, waits for one semantic
action, pulls that event, and requests clean close. It needs neither Node.js nor
development-machine certificate provisioning.

~~~powershell
cargo build --release --manifest-path native/Cargo.toml -p anodrel-native-ui-client-sample
$uiClientPath = (Resolve-Path native/target/release/anodrel-native-ui-client-sample.exe).Path
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --native-ui-sample-client $uiClientPath
~~~

When the Anodrel window appears, activate **Complete native UI diagnostic** by
clicking it or using Tab then Enter. The window closes only after the child has
received its own revision-bound action and sent `session.close`; the host then
prints **Anodrel native UI development probe completed successfully.** Closing
the window instead is a safe manual abort, not a passing diagnostic. The child
has one two-minute action wait, while the host stops it after a bounded post-
window-close wait so a manual abort leaves no child behind.

### Windows end-to-end Node development sample

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

### Live-status screen-reader check

The same client has a deliberate v3 status diagnostic. Start the window, then
start Narrator **after** it opens. Activate **Publish visible result**. Narrator
should announce the later polite result and then the later assertive result;
the sample closes after each has remained visible for three seconds.

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --sample-ui-live-status-client $nodePath $clientPath
~~~

Inspect or Accessibility Insights should show the status as a `Text` element
with `LiveSetting` `Polite` first and `Assertive` second. The application
receives only accepted document revisions and cannot learn whether either
announcement was delivered. See `docs/UI_LIVE_ANNOUNCEMENTS.md`.

To exercise the direct Windows session-menu path, run:

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --sample-ui-menu-client $nodePath $clientPath
~~~

The **Anodrel UI Session Lab** publishes a document with no ordinary action,
then shows a native **File & actions** menu. First choose **Complete & close**.
In a second run, press **Ctrl+Shift+M** instead. Each run should close only
after its current User32 route reached the host's private mapping, crossed the
shared mailbox, and returned as the exact `menu.action.invoked` pull event.
The literal ampersands must remain visible: the application cannot declare an
`Alt` mnemonic. The shortcut must not trigger while held, with a different
modifier state, or outside the window. This is the manual acceptance check for
the direct menu adapter; the client learns no menu handle, command number,
keyboard state, focus, opening, or dismissal state.

To exercise the actual host-owned open or save picker in that same session,
replace `--sample-ui-client` with `--sample-ui-file-client` or
`--sample-ui-save-client`. Both commands accept only the strict sample filters;
the save diagnostic never writes the selected destination.

To exercise the separately granted selection-scoped write route, run:

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --sample-ui-file-write-client $nodePath $clientPath
~~~

Choose a new temporary `.txt` filename in the host-owned save picker. The
client receives one opaque save reference and writes a fixed diagnostic line
through it; inspect the file after the session closes. Cancelling leaves no new
file behind. This route is deliberately distinct from `--sample-ui-save-client`,
which remains a non-mutating selection test.

To exercise the separately granted bounded binary-output route, run:

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --sample-ui-file-binary-write-client $nodePath $clientPath
~~~

Choose a new temporary `.bin` filename in the host-controlled save picker. The
client writes exactly the fixed bytes `41 6E 6F 64 72 65 6C 00 FF` through one
opaque save reference; inspect the file after the session closes. Cancelling
leaves no new file behind. This route has its own `file.write_binary` grant and
does not accept a path, type, handle, offset, stream, or readback request. This
manual check is not yet recorded as passed; see `docs/FILE_BINARY_WRITE.md`.

To exercise the bounded application-state service through the same authenticated
session, run:

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --sample-ui-storage-client $nodePath $clientPath
~~~

The diagnostic derives the fixed `anodrel.sample` development identity on the
host, replaces its state with a fixed test snapshot, reads that exact snapshot,
and clears it before waiting for the regular semantic action and closing its
own session. It accepts no application-supplied path and leaves no saved test
snapshot behind. This is a development diagnostic; installed-application
policy integration remains separate work.

To exercise the bounded authenticated diagnostic-log read, run:

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --sample-ui-diagnostics-client $nodePath $clientPath
~~~

The development client verifies exactly the fixed `core` and `transport` host
events, then completes the regular semantic UI action. It has no path, free
text, filter, write, clear, export, or subscription surface.

To exercise the exact Windows Credential Manager boundary through the same
authenticated session, run:

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --sample-ui-credentials-client $nodePath $clientPath
~~~

The development client writes, reads, and deletes one process-scoped secret
from the host-bound `anodrel.sample` namespace before it waits for the normal
semantic UI action. The pipe worker performs the synchronous store work; the
UI thread does not. This is a development diagnostic, not a product credential
session; it does not show or log the test value.

To exercise the bounded notification boundary through the same authenticated
session, run:

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --sample-ui-notification-client $nodePath $clientPath
~~~

The client delivers its document and then calls `notification.show` once. Watch
for an **Anodrel notification diagnostic** notification from the notification
area; the entry carries the Anodrel brand icon and appears only once the first
notification is sent. Then complete the normal semantic action to close the
session, which removes the entry.

This check has passed on Windows 11: the notification appeared with the client's
title and body, and its body line feed rendered as a line break. Windows
attributes it to `anodrel-windows-host.exe`, which is expected — Shell32 has no
application identity to show instead.

Two things this check cannot tell you, by design. The client learns only that
the host accepted the values, so a notification you have silenced or muted still
reports success — that is the privacy line in `docs/NOTIFICATIONS.md`, not a
fault. And if nothing appears, check Windows notification settings for this
application before suspecting the host.

If the operating system refuses outright, the client stops at safe stage 24
within about a second rather than waiting for the action.

To exercise the first public window capability through the same authenticated
session, run:

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --sample-ui-window-title-client $nodePath $clientPath
~~~

The client delivers its document and then calls `window.title.set` once,
proposing **Windows Security** — a name that would be a lie on its own. The
window's caption must read:

~~~text
Windows Security — Anodrel Sample
~~~

That is the whole check. The application supplied the first half and cannot
supply, suppress, or forge the second, so a window can say what it is showing
and never change what it is. Look at the taskbar and Alt+Tab as well as the
title bar: those are the surfaces the guarantee exists for. Then complete the
normal semantic action to close the session.

This check has passed on Windows 11 with exactly that caption. If the client
stops at safe stage 25 the host refused the proposal; a caption missing its
suffix would mean the session had no validated display name, which for this
sample would be a defect rather than the documented fallback.

See `docs/WINDOW_TITLE.md` for what this capability deliberately does not do —
there is no window target, no read, and no other window property.

To exercise the separately granted window-state command through the same
authenticated session, run:

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --sample-ui-window-state-client $nodePath $clientPath
~~~

The client requests **minimized**, **maximized**, then **restored**, holding
each state briefly so it is visible. It receives acceptance only; it never
learns the window's current state, bounds, monitor, focus, or handle. After the
window is restored, complete the normal semantic action to close the session.
No other Anodrel window should change. If the client stops at safe stage 28, one
of the closed commands was not accepted.

This manual check is not yet recorded as passed. See `docs/WINDOW_STATE.md` for
the deliberately absent targeting, geometry, focus, and readback APIs.

To exercise the separately granted session-window foreground request, run:

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --sample-ui-window-focus-client $nodePath $clientPath
~~~

The client gives you about 1.5 seconds to bring another application to the
foreground, then sends exactly one `window.focus.request`. Observe what Windows
does: it may foreground the Anodrel window or flash its taskbar instead. Either
is a valid operating-system policy outcome, and the client must not learn which
one occurred. Complete the normal semantic action to close the session. If the
client stops at safe stage 29, Windows refused the request or the UI session was
unavailable; the host deliberately does not distinguish them.

This manual check is not yet recorded as passed. See `docs/WINDOW_FOCUS.md` for
the deliberately absent target, focus readback, input, retry, and foreground
policy bypass APIs.

To exercise the separately granted reversible fullscreen command, run:

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --sample-ui-window-fullscreen-client $nodePath $clientPath
~~~

The client requests borderless `fullscreen`, holds it briefly, then requests
`windowed`. Watch the Anodrel window fill the monitor it already occupies and
return to its original framed placement. It never receives the monitor, bounds,
style, display mode, or current fullscreen state. On a multi-monitor desktop,
move the window to a non-primary monitor before the first request and confirm
that same monitor is used. Then complete the normal semantic action to close
the session. If the client stops at safe stage 30, the host could not safely
apply or restore the mode; it deliberately does not reveal which native step
failed.

This manual check is not yet recorded as passed. See `docs/WINDOW_FULLSCREEN.md`
for the deliberately absent monitor selection, exclusive display control,
geometry, state readback, event, and cross-window APIs.

To exercise the separately granted bounded client-size command, run:

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --sample-ui-window-size-client $nodePath $clientPath
~~~

The client requests an 800 by 520 logical client area for its own session
window. At both 100% and a non-100% display scale, observe that the window
resizes without moving, activating, or changing z-order; it never receives the
outer bounds, monitor, DPI, or resulting size. Complete the normal semantic
action to close the session. If the client stops at safe stage 32, the host
could not safely apply the request and deliberately does not reveal why.

To verify the fullscreen boundary separately, run:

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --sample-ui-window-size-fullscreen-client $nodePath $clientPath
~~~

This route enters reversible fullscreen, expects `window.size.set` to fail
only with `window.unavailable`, then restores the original presentation. If it
stops at safe stage 33, the expected refusal or safe restoration did not occur.

This manual check is not yet recorded as passed. See `docs/WINDOW_SIZE.md` for
the deliberately absent target, position, monitor, DPI, bounds, constraint,
animation, event, and readback APIs.

To exercise text fields and the granted value read, run:

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --sample-ui-fields-client $nodePath $clientPath
~~~

The client publishes a document with two fields — **Name**, empty, and
**Note**, pre-filled with `edit me` — then immediately reads them once and
prints that it got back exactly what it set.

Now type. Tab to **Name** and enter something, Tab to **Note** and change it,
then activate **Submit field values**. The client reads a second time and prints
both values.

The gap between those two reads is the whole point: everything you typed
happened without the application learning anything. There is no change event to
subscribe to, and the second read only happened because a person activated an
action. If the client stops at safe stage 26, its first read disagreed with the
document it had just set.

This check has passed on Windows 11: both fields were reachable by click and by
Tab, typing and editing worked, and after **Submit field values** the window
showed each field's exact text under *RECEIVED BY THE APPLICATION*, including an
edit made to a pre-filled value. The host exited cleanly with no error output.

That run also found the wrapping gap: the closing sentence was cut off mid-word
at the window edge, because a text run did not reflow. It now wraps to the
column (Decision 0068), and the sample's closing sentence is deliberately long
so this check exercises it. Resize the window and the paragraph should reflow
with nothing lost at either edge.

See `docs/UI_FIELDS.md` for what a read carries and what it deliberately does
not — no caret, selection, timing, or edited flag.

To exercise a version 2 scroll document through that same session, run:

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --sample-ui-scroll-client $nodePath $clientPath
~~~

Use the mouse wheel, Page Down, the scrollbar track, or its draggable thumb
until **Complete scroll diagnostic** becomes visible, then activate it. The
document carries no position: the host retains, clamps, and applies the
vertical offset locally before the normal authenticated semantic-action round
trip closes the session. The scrollbar interaction must not focus another
element or emit an action by itself. The development client waits at most two
minutes for the action.

### Development Windows product fixture

This is the only path that exercises the complete verified product session:
machine policy, locked digest revalidation, Authenticode publisher match,
child-only bootstrap delivery, authenticated pipe, host-owned native window,
one semantic action, and coordinated shutdown.

It is a **development-machine** procedure. Provisioning installs a locally
generated code-signing certificate into the machine root and trusted-publisher
stores and writes one `HKEY_LOCAL_MACHINE` policy record. Both need an elevated
PowerShell session, and both are reversed by `-Remove`. Read
`docs/PRODUCT_FIXTURE.md` before running it.

From an **elevated** PowerShell session at the repository root:

~~~powershell
.\scripts\provision-product-fixture.ps1
~~~

The script builds the fixture and its provisioning helper, stages a package
under `%LOCALAPPDATA%\Anodrel\ProductFixture`, creates or reuses the development
certificate, signs the staged executable, installs machine trust, and writes the
record. It ends by reporting that the machine record validates.

To check the current state at any time — including before provisioning anything
— use the query-only switch, which changes nothing and needs no elevation:

~~~powershell
.\scripts\provision-product-fixture.ps1 -Verify
~~~

Then, from an ordinary session:

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --product-session org.anodrel.product-fixture
~~~

Confirm each of the following:

1. an **Anodrel Product Session** window opens on a host-owned waiting screen;
2. it is replaced by the fixture's document, headed *Signed child, authenticated
   window*;
3. **Complete product session** responds to hover, and Tab plus Enter reaches it;
4. activating it closes the window within a moment — that is the fixture's
   `session.close` reaching the host-owned close signal, not the window manager;
5. the host process exits; and
6. `anodrel-product-fixture.exe` is gone from Task Manager.

Also check the two failure paths. Close the window with its title-bar button
instead of activating the action: the window must close, the host must exit, and
the child must still disappear. Separately, end `anodrel-product-fixture.exe`
from Task Manager while the window is open: the window must close on its own.

There is a third path worth checking from the Startup Lab, because a launch
takes a noticeable moment. Click **Development Fixture** and immediately close the
Startup Lab window, before the product window appears. The host must exit and
`anodrel-product-fixture.exe` must not be left running: a session that finishes
starting after its surface has gone is ended by the host rather than handed to a
window that no longer exists.

The Startup Lab reads the same provisioning state:

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --showcase apps/sample/anodrel.application.json
~~~

With the fixture provisioned, **Development Fixture** is drawn live, reads
*Development only, not a product*, responds to hover, and opens a window titled
**Anodrel Development Product Fixture**. Run
`.\scripts\provision-product-fixture.ps1 -Remove` and repeat: the tile must
return to *Not provisioned*, dimmed and marked **PLANNED**, and ignore clicks.

Remove the fixture when you are finished:

~~~powershell
.\scripts\provision-product-fixture.ps1 -Remove
~~~

The protocol half of this path is covered automatically and needs no
provisioning:

~~~text
cargo test --manifest-path native/Cargo.toml -p anodrel-product-fixture
~~~

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
