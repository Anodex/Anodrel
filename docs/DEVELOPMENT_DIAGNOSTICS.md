# Anodrel development diagnostics

## Crash records

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

## Compiled native development probe

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

## Compiled native HTTPS diagnostic

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

## Compiled native UI-session diagnostic

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

## Windows end-to-end Node development sample

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

## Live-status screen-reader check

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

To exercise the independently granted host-owned folder picker, run:

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --sample-ui-folder-client $nodePath $clientPath
~~~

Select one local filesystem folder. In a second run, cancel the picker. In
each case, activate the sample's ordinary action to close the session. The
selected result is only one bounded display path: the diagnostic cannot set the
title, initial folder, filters, or flags, and it receives no directory access,
enumeration, handle, or retained permission. This is the manual acceptance
check for `dialog.open_folder`; see `docs/FOLDER_DIALOGS.md`.

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

To exercise the separate pull-only state observation, run:

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --sample-ui-window-state-read-client $nodePath $clientPath
~~~

The client first confirms that its newly created session window is **restored**.
It then requests **maximized** and **restored**, checking an immediate snapshot
after each request, before waiting for the normal semantic action to close the
session. The operator can see both transitions, but the client receives only
the three portable state names. A safe stage-37 stop means the host could not
return the expected state for that session's own window. No other Anodrel
window should change. See `docs/WINDOW_STATE_OBSERVATION.md`.

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

## Development Windows product fixture

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

