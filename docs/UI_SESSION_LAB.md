# Windows UI Session Lab

**Status:** Development diagnostic. This lab proves one authenticated UI
document can move from the private Windows pipe to one host-controlled native
window. It is not a product application window or a general session launcher.

## Run

The Node.js development client exercises the broader service path. Build it,
then pass explicit paths to a locally installed Node executable and the
compiled client:

~~~text
npm run build
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --sample-ui-client path-to-node.exe apps/sample/dist/native-client.js
~~~

The compiled native UI diagnostic exercises the foundational interactive path
without Node.js or development-machine certificate provisioning:

~~~text
cargo build --release --manifest-path native/Cargo.toml -p anodrel-native-ui-client-sample
$uiClientPath = (Resolve-Path native/target/release/anodrel-native-ui-client-sample.exe).Path
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --native-ui-sample-client $uiClientPath
~~~

Activate **Complete native UI diagnostic** in the resulting window. Its child
submits only one compiled-in document, accepts only its own action at revision
1, and closes only its own session; it is a diagnostic, not a general native
application or window API.

To repeat the separate host-only Windows UI Automation Invoke acceptance check
for that same compiled child, follow `docs/UI_AUTOMATION_INVOKE_PROBE.md` or
double-click `start-uia-invoke-probe.bat`. It invokes the one fixed button
without a local click and passes only after the child receives its normal
revision-bound event and closes its session.

The constrained generated native template uses the same host-controlled UI-session
mechanics, but is created in a new directory by `anodrel-native-app-tool`.
Follow `docs/NATIVE_UI_TEMPLATE.md` to create, build, and run it through
`--native-template-client`. That explicit route is also development-only: it
accepts an operator-selected executable and grants exactly document replacement,
semantic-action reading, and self-close.

The separate constrained native window-controls template uses the same
authenticated-session mechanics but exercises the existing typed targetless
controls for that one session's host window. Follow
`docs/NATIVE_WINDOW_CONTROLS_TEMPLATE.md` to create, build, and run it through
`--native-window-controls-template-client`. Its explicit development route
grants exactly document replacement, semantic-action reading, title proposal,
state, focus, fullscreen, bounded client size, and self-close. It gives the
project no native handle, window target, geometry, display, focus, or title
readback, and it has no product-launch authority.

The separate constrained native multi-window template uses the same
authenticated-session mechanics but creates a bounded session-owned view group.
Follow `docs/NATIVE_MULTI_WINDOW_TEMPLATE.md` to create, build, and run it
through `--native-multi-window-template-client`. Its explicit development route
grants exactly document replacement, tagged semantic-action reading, secondary
open, secondary close, and self-close. It does not give the project a native
handle, geometry, title readback, enumeration, or product-launch authority.

The separate constrained native form template uses the same authenticated
session mechanics but demonstrates host-owned text entry and one intentional
field snapshot. Follow `docs/NATIVE_FORM_TEMPLATE.md` to create, build, and
run it through `--native-form-template-client`. Its explicit development route
grants exactly document replacement, semantic-action reading, whole-surface
field reading, and self-close. It does not give the project keystrokes, input
timing, a field selector, focus/caret/selection state, native control, handle,
or product-launch authority.

The separate constrained native live-status template uses the same
authenticated-session mechanics but demonstrates three fixed complete version-3
status documents. Follow `docs/NATIVE_LIVE_STATUS_TEMPLATE.md` to create,
build, and run it through `--native-live-status-template-client`. Its explicit
development route grants exactly document replacement, semantic-action reading,
and self-close. It receives no accessibility listener state, announcement
delivery result, callback, notification, UI Automation identifier, or native
event authority.

To exercise Protocol 1.7's UI-thread-routed open picker in the same
authenticated session, use `--sample-ui-file-client` in place of
`--sample-ui-client`. Select a `.txt`, `.json`, or `.md` file or cancel the
picker, then activate the visible semantic action to end the sample.

To exercise Protocol 1.8's independent save picker, use
`--sample-ui-save-client` instead. Choose a `.txt`, `.json`, or `.md`
destination or cancel it, then activate the visible semantic action. The
diagnostic verifies picker routing only: it never creates, truncates, or writes
the selected destination.

To exercise the owned version 2 scroll path, use
`--sample-ui-scroll-client` instead. Scroll with the mouse wheel or Page Down
until **Complete scroll diagnostic** appears, then activate it. The action is
not initially visible, so the run proves that the host retained and applied a
local viewport offset before it sent the ordinary revision-bound semantic event.

The window starts with an Anodrel-owned waiting document. Each private client
receives its one-time invitation through standard input, authenticates to a
current-session named pipe, then submits one strict `anodrel.ui.document.v1`
document using `ui.document.replace`, or an exact
`anodrel.ui.document.v2` document using `ui.document.replace.v2`. The native
window must replace the waiting screen with that document. The regular and
scroll diagnostics close only after their expected semantic action has returned
through `ui.events.read`; closing the native window is the safe manual abort.
The development client waits at most two minutes for that action. It paces that
wait by backoff rather than a fixed interval — 25 ms growing by half to a
one-second cap — so an immediate click is still answered promptly while an open
window does not cost a constant stream of `ui.events.read` round trips. The
compiled native diagnostic and native product fixture use the same first-party
schedule.

## Boundary

The host gives this one pipe session one `UiDocumentMailbox`. Its worker thread
publishes only the latest accepted snapshot; the Windows UI thread polls that
mailbox and redraws only when the revision advances. The lab reads no document
file, package, URL, asset, or policy from the client, and it never shows the
pipe name, token, bootstrap material, raw path, or native error.

The lab maps pointer hit tests and Tab/Shift+Tab/Enter only into bounded
revision-and-action candidates. The sample client reads them through
`ui.events.read`, which revalidates them in the authenticated session before
returning a `ui.action.invoked` event. An action still has no native operation
or capability meaning. The lab has no accessibility adapter, unsolicited event
delivery, callback, or background task. Its development sample can request a
close only for that same authenticated session through the separate
`session.close` capability; the UI thread observes that host-owned signal and
closes its one lab window. See `docs/UI_SESSIONS.md`, `docs/TRANSPORT.md`, and
Decisions 0035 and 0036.
