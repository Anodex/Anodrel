# Windows UI Session Lab

**Status:** Development diagnostic. This lab proves one authenticated UI
document can move from the private Windows pipe to one host-controlled native
window. It is not a product application window or a general session launcher.

## Run

Build the TypeScript development client first, then pass explicit paths to a
locally installed Node executable and the compiled client:

~~~text
npm run build
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --sample-ui-client path-to-node.exe apps/sample/dist/native-client.js
~~~

To exercise Protocol 1.7's actual UI-thread-routed file picker in the same
authenticated session, use `--sample-ui-file-client` in place of
`--sample-ui-client`. Select a `.txt`, `.json`, or `.md` file or cancel the
picker, then activate the visible semantic action to end the sample.

The window starts with an Anodrel-owned waiting document. The private client
receives its one-time invitation through standard input, authenticates to a
current-session named pipe, checks `platform.health`, then submits one strict
`anodrel.ui.document.v1` document using `ui.document.replace`, or an exact
`anodrel.ui.document.v2` document using `ui.document.replace.v2`. The native
window must replace the waiting screen with that document. Close the window to
complete the development run.

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
