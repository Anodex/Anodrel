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

The window starts with an Anodrel-owned waiting document. The private client
receives its one-time invitation through standard input, authenticates to a
current-session named pipe, checks `platform.health`, then submits one strict
`anodrel.ui.document.v1` document using `ui.document.replace`. The native
window must replace the waiting screen with that document. Close the window to
complete the development run.

## Boundary

The host gives this one pipe session one `UiDocumentMailbox`. Its worker thread
publishes only the latest accepted snapshot; the Windows UI thread polls that
mailbox and redraws only when the revision advances. The lab reads no document
file, package, URL, asset, or policy from the client, and it never shows the
pipe name, token, bootstrap material, raw path, or native error.

Visible actions in the delivered document are deliberately inert: the lab has
no pointer, keyboard, accessibility, or application event bridge. It does not
grant a capability or execute a native operation. See `docs/UI_SESSIONS.md`,
`docs/TRANSPORT.md`, and Decision 0034.
